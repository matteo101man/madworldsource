# Native exception transfer graph

Bounded original functions802e45dc,802e4ae8,8021f368; native-instruction nodes retain decoded operands; native-control-transfer edges preserve link bits. No raw words or decompiled C++ in queries. Select control-transfer and special-register writes to locate nonlocal transfer boundaries. This does not prove full exception ABI.
