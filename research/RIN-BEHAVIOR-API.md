# Native behavior query

`native-function --calls--> native-api` records direct calls and native instruction addresses. `native-function --loads--> native-resource` records literal resource arguments to ResourceManager::GetData. These prove resource requests in function bodies, not branch eligibility or runtime timing. Analysis addresses use the relocated base in enemy-link.json.

Query: `python -m gk graph research/rinrin-behavior.graph.jsonl digest`. Detailed dependency query: `python tools/query_rin.py . 810cb74c`.
