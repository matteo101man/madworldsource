"""Serialize local typed records in the queryable gk graph format."""
import sys
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
from gk.graph import Graph

def save_graph(path,records,authority):
    graph=Graph(dict(game='MadWorld',authority=authority));ids=set()
    for row in records:
        if row['type']!='node':continue
        if row['id'] in ids:raise ValueError('Duplicate graph node')
        ids.add(row['id']);attrs={k:v for k,v in row.items() if k not in ('type','id','kind','label')}
        graph.add_node(row['id'],row['kind'],row.get('label',row.get('name',row['id'])),**attrs)
    for row in records:
        if row['type']!='edge':continue
        if row['source'] not in ids or row['target'] not in ids:raise ValueError('Unresolved graph edge')
        attrs={k:v for k,v in row.items() if k not in ('type','source','target','kind')}
        graph.add_edge(row['source'],row['target'],row['kind'],**attrs)
    graph.save(path);loaded=Graph.load(path)
    if len(loaded.nodes)!=len(ids) or len(loaded.edges)!=len(graph.edges):raise ValueError('Graph round-trip mismatch')
    return graph
