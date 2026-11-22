"""Logic graph: a small typed graph for game scripts, data tables and their relationships.

Why a graph and not a decompiled text dump: an LLM (or a person) reads a 400-script game by following relationships,
not by reading 10 MB top to bottom. Nodes carry a stable id and a one-line label; edges are explicit and typed, so
"who calls this", "what does this section trigger" and "which scripts load which" are queries, not greps.

Conventions (decoder-agnostic; a game's decoder picks its own node/edge kinds):
  node id    "<kind>:<path or name>[#section][@offset]"  stable across runs, human-greppable
  edge kinds "contains" (ordered by attr `seq`), "calls", "invokes", "branch" (attr `role`), "refs", "requires", ...
Storage: JSON Lines. Line 1 `{"gk_graph": 1, "meta": {...}}`, then `{"n": {...}}` / `{"e": {...}}` records.
"""
import json
from collections import Counter, defaultdict, deque


class Graph:
    def __init__(self, meta=None):
        self.meta = dict(meta or {})
        self.nodes = {}
        self.edges = []
        self._out = defaultdict(list)
        self._in = defaultdict(list)

    # ------------------------------------------------------------------ build
    def add_node(self, nid, kind, label="", **attrs):
        n = self.nodes.get(nid)
        if n is None:
            n = {"id": nid, "kind": kind, "label": label}
            if attrs:
                n["attrs"] = dict(attrs)
            self.nodes[nid] = n
        else:
            if label and not n.get("label"):
                n["label"] = label
            if attrs:
                n.setdefault("attrs", {}).update(attrs)
        return nid

    def add_edge(self, src, dst, kind, **attrs):
        e = {"src": src, "dst": dst, "kind": kind}
        if attrs:
            e["attrs"] = dict(attrs)
        self.edges.append(e)
        i = len(self.edges) - 1
        self._out[src].append(i)
        self._in[dst].append(i)
        return e

    # ------------------------------------------------------------------ persistence
    def save(self, path):
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(json.dumps({"gk_graph": 1, "meta": self.meta}, ensure_ascii=False) + "\n")
            for n in self.nodes.values():
                f.write(json.dumps({"n": n}, ensure_ascii=False, separators=(",", ":")) + "\n")
            for e in self.edges:
                f.write(json.dumps({"e": e}, ensure_ascii=False, separators=(",", ":")) + "\n")

    @classmethod
    def load(cls, path):
        g = None
        with open(path, encoding="utf-8") as f:
            for ln, line in enumerate(f, 1):
                rec = json.loads(line)
                if g is None:
                    if "gk_graph" not in rec:
                        raise ValueError("%s: not a gk graph (first line must carry gk_graph)" % path)
                    g = cls(rec.get("meta"))
                elif "n" in rec:
                    n = rec["n"]
                    g.nodes[n["id"]] = n
                elif "e" in rec:
                    e = rec["e"]
                    if e["src"] not in g.nodes or e["dst"] not in g.nodes:
                        raise ValueError("%s:%d: edge references an unknown node" % (path, ln))
                    g.edges.append(e)
                    g._out[e["src"]].append(len(g.edges) - 1)
                    g._in[e["dst"]].append(len(g.edges) - 1)
        if g is None:
            raise ValueError("%s: empty file" % path)
        return g

    # ------------------------------------------------------------------ queries
    def out(self, nid, kinds=None):
        """Outgoing edges as (edge, target node)."""
        return [(self.edges[i], self.nodes[self.edges[i]["dst"]]) for i in self._out.get(nid, ())
                if kinds is None or self.edges[i]["kind"] in kinds]

    def inn(self, nid, kinds=None):
        return [(self.edges[i], self.nodes[self.edges[i]["src"]]) for i in self._in.get(nid, ())
                if kinds is None or self.edges[i]["kind"] in kinds]

    def children(self, nid):
        """Targets of `contains` edges in `seq` order."""
        kids = self.out(nid, ("contains",))
        kids.sort(key=lambda t: t[0].get("attrs", {}).get("seq", 0))
        return [n for _, n in kids]

    def find(self, text, kinds=None, limit=50):
        t = text.lower()
        hits = []
        for n in self.nodes.values():
            if kinds and n["kind"] not in kinds:
                continue
            hay = n["id"] + " " + n.get("label", "") + " " + json.dumps(n.get("attrs", {}), ensure_ascii=False)
            if t in hay.lower():
                hits.append(n)
                if len(hits) >= limit:
                    break
        return hits

    def ego(self, nid, depth=1, kinds=None, direction="both"):
        """Subgraph within `depth` hops of `nid`, following only edges of `kinds` (all when None)."""
        if nid not in self.nodes:
            raise KeyError(nid)
        seen, q = {nid: 0}, deque([nid])
        keep = set()
        while q:
            cur = q.popleft()
            if seen[cur] >= depth:
                continue
            nxt = []
            if direction in ("both", "out"):
                nxt += [(i, self.edges[i]["dst"]) for i in self._out.get(cur, ())]
            if direction in ("both", "in"):
                nxt += [(i, self.edges[i]["src"]) for i in self._in.get(cur, ())]
            for i, other in nxt:
                if kinds and self.edges[i]["kind"] not in kinds:
                    continue
                keep.add(i)
                if other not in seen:
                    seen[other] = seen[cur] + 1
                    q.append(other)
        sub = Graph(dict(self.meta, ego=nid, depth=depth))
        for n in seen:
            sub.nodes[n] = self.nodes[n]
        for i in sorted(keep):
            e = self.edges[i]
            sub.edges.append(e)
            sub._out[e["src"]].append(len(sub.edges) - 1)
            sub._in[e["dst"]].append(len(sub.edges) - 1)
        return sub

    def path(self, a, b, kinds=None):
        """Shortest directed path a -> b as a list of node ids, or None."""
        prev, q = {a: None}, deque([a])
        while q:
            cur = q.popleft()
            if cur == b:
                out = []
                while cur is not None:
                    out.append(cur)
                    cur = prev[cur]
                return out[::-1]
            for e, n in self.out(cur, kinds):
                if n["id"] not in prev:
                    prev[n["id"]] = cur
                    q.append(n["id"])
        return None

    # ------------------------------------------------------------------ summaries
    def stats(self):
        return {"nodes": len(self.nodes), "edges": len(self.edges),
                "node_kinds": dict(Counter(n["kind"] for n in self.nodes.values())),
                "edge_kinds": dict(Counter(e["kind"] for e in self.edges))}

    def digest(self, top=15):
        """Markdown overview meant to be the first thing an agent reads."""
        s = self.stats()
        L = ["# Graph digest", "", "%d nodes, %d edges." % (s["nodes"], s["edges"]), "",
             "Nodes: " + ", ".join("%s %d" % kv for kv in sorted(s["node_kinds"].items())),
             "Edges: " + ", ".join("%s %d" % kv for kv in sorted(s["edge_kinds"].items())), ""]
        for kind in sorted(s["node_kinds"]):
            ranked = sorted((n for n in self.nodes.values() if n["kind"] == kind),
                            key=lambda n: -len(self._in.get(n["id"], ())))[:top]
            ranked = [n for n in ranked if self._in.get(n["id"])]
            if ranked:
                L.append("## Most referenced %s" % kind)
                for n in ranked:
                    kinds = Counter(self.edges[i]["kind"] for i in self._in[n["id"]])
                    L.append("- `%s` %s (%s)" % (n["id"], n.get("label", ""), ", ".join("%s x%d" % kv for kv in kinds.most_common(3))))
                L.append("")
        return "\n".join(L)


# ---------------------------------------------------------------------- renderers
def _lab(n, width=70):
    s = (n.get("label") or n["id"]).replace("\n", " ")
    return s if len(s) <= width else s[:width - 1] + "..."


def outline(g, root, depth=3, extra_kinds=("calls", "invokes", "requires", "refs"), max_children=60):
    """Indented text view of the `contains` tree under `root`, annotating other outgoing edges inline."""
    L = []

    def walk(nid, d):
        n = g.nodes[nid]
        notes = []
        for e, t in g.out(nid):
            if e["kind"] in extra_kinds:
                notes.append("%s %s" % (e["kind"], t["id"]))
        L.append("%s%s  [%s]%s" % ("  " * d, _lab(n), nid, ("  -> " + "; ".join(notes[:4])) if notes else ""))
        if d >= depth:
            return
        kids = g.children(nid)
        for k in kids[:max_children]:
            walk(k["id"], d + 1)
        if len(kids) > max_children:
            L.append("%s... %d more" % ("  " * (d + 1), len(kids) - max_children))

    walk(root, 0)
    return "\n".join(L)


def _mid(nid):
    return "n" + "".join(c if c.isalnum() else "_" for c in nid)


def mermaid(g, max_nodes=80):
    ids = list(g.nodes)[:max_nodes]
    keep = set(ids)
    L = ["graph LR"]
    for nid in ids:
        L.append('  %s["%s"]' % (_mid(nid), _lab(g.nodes[nid], 40).replace('"', "'")))
    for e in g.edges:
        if e["src"] in keep and e["dst"] in keep:
            L.append("  %s -->|%s| %s" % (_mid(e["src"]), e["kind"], _mid(e["dst"])))
    return "\n".join(L)


def dot(g, max_nodes=200):
    ids = list(g.nodes)[:max_nodes]
    keep = set(ids)
    L = ["digraph G {", "  rankdir=LR;"]
    for nid in ids:
        L.append('  "%s" [label="%s"];' % (nid.replace('"', "'"), _lab(g.nodes[nid], 40).replace('"', "'")))
    for e in g.edges:
        if e["src"] in keep and e["dst"] in keep:
            L.append('  "%s" -> "%s" [label="%s"];' % (e["src"].replace('"', "'"), e["dst"].replace('"', "'"), e["kind"]))
    L.append("}")
    return "\n".join(L)


# ---------------------------------------------------------------------- cli
def main(args):
    g = Graph.load(args.file)
    cmd = args.gcmd
    if cmd == "stats":
        print(json.dumps(g.stats(), indent=2))
    elif cmd == "digest":
        print(g.digest())
    elif cmd == "search":
        for n in g.find(args.text, args.kind and set(args.kind.split(",")), args.limit):
            print("%s  %s" % (n["id"], _lab(n, 100)))
    elif cmd == "outline":
        print(outline(g, args.node, args.depth))
    elif cmd == "ego":
        sub = g.ego(args.node, args.depth, args.kinds and set(args.kinds.split(",")), args.direction)
        print({"mermaid": mermaid, "dot": dot}[args.format](sub) if args.format in ("mermaid", "dot") else
              "\n".join(json.dumps(x, ensure_ascii=False) for x in list(sub.nodes.values()) + sub.edges))
    elif cmd in ("callers", "callees"):
        f = g.inn if cmd == "callers" else g.out
        for e, n in f(args.node, set(args.kinds.split(",")) if args.kinds else None):
            print("%s  %s  (%s)" % (n["id"], _lab(n, 80), e["kind"]))
    elif cmd == "path":
        p = g.path(args.a, args.b)
        print(" -> ".join(p) if p else "no path")
    return 0
