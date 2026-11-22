# Native static collision API

`collision-resource contains collision-body`, `collision-body shape collision-shape`, and nested `contains`/`accelerated-by` edges describe original Havok ownership. `research/collision.graph.jsonl` is queryable with `python -m gk graph PATH digest`. Original reflected types/member offsets are indexed in `havok-schema.json` and its graph. Source SHA-256 values are on resource nodes.

Five loader requests in DOL 0x802140e4 call 0x800af3ec: col.hkx(category0x16), col_cm.hkx(0x11), col_ef.hkx(0x15), col_obj.hkx(0x14), col_pl.hkx(0x13). These arguments remain neutral categories: their filter semantics are not inferred. Original body filterInfo is retained separately, even when zero.

353 bodies,362 leaf shapes,3547 vertices,5642 triangles; no unsupported reachable shapes. MOPP code is fingerprinted, while child geometry is staged. Convex transform/translate/list wrappers are traversed. Source reflected SimpleMeshShape inheritance and member layouts override upstream placeholder types in-process. The actual files use Havok5.5.0-r1 32-bit big-endian packfiles. Tools remain outside the source repository.

`mw_data::collision::CollisionWorld::parse`: validates finite values, unit planes, triangle bounds, original/derived topology distinction, category/leaf ownership and staging failures before runtime use.

`mw_sim::collision::raw_surface_ray`: unit-direction raw-surface diagnostic, sorted hits with body/shape provenance. Convex halfspaces define geometry; derived hull triangles are visualization only. Mesh queries are double-sided. A ray starting inside a convex shape returns the exit surface. Optional source argument chooses a resource, not a native filter. This API does not apply native shape radius, welding, material response, contact tolerance, MOPP queries, CCD or solver behavior. Do not use it as evidence of Havok equivalence.

Independent Python and Rust roof probes agree: beneath RinRin's original EMS spawn, col.hkx/cube3219_ST0c has raw top plane y17.54882725887; initial spawn is0.2740487 above it, shape radius0.05. Do not force model feet to this plane: original initial placement versus settled physics is still open.

Focused pseudocode: read original HKX reflection; unpack each body and rigid transform; traverse enabled native children; transform vertices and convex planes into world space with hkTransform's implicit affine last row; retain radii/material/welding data; reject unsupported reachable shapes; validate staged geometry; probe independently. This is staging/diagnostic policy, not translated native Havok simulation.
