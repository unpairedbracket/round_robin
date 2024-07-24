from typing import Literal

import networkx as nx

class Graph:
    def __init__(self):
        self.graph = nx.DiGraph()
        self.add_node('s')
        self.add_node('t')
        self.candidates = 0
        self.matches = 0

    def add_node(self, nid, name: str | None = None, layer: Literal["s", "m", "c", "pre-t", "t"] | None = None):
        if name is None:
            name = nid
        if layer is None:
            if nid == 's':
                layer = '0-s'
            elif isinstance(nid, tuple):
                layer = '1-m'
                self.matches += 1
            elif isinstance(nid, int):
                self.candidates += 1
                layer = '2-c'
            elif nid == 'pre-t':
                layer = '3-t'
            elif nid == 't':
                layer = '4-t'
        self.graph.add_node(nid, label=name, layer=layer)

    def add_edge(self, frm, to, capacity=None):
        if capacity is None:
            kw = {}
        else:
            kw = {'capacity': capacity}
        self.graph.add_edge(frm, to, **kw)

    def draw_graph(self):
        pos = nx.multipartite_layout(self.graph, subset_key='layer')
        for node, position in pos.items():
            if self.graph.nodes[node]['layer'] == '2-c':
                position[1] *= (self.matches / self.candidates)
        nx.draw_networkx(self.graph, pos, node_size=2000)

    def can_be_satisfied(self):
        max_flow, resulting_flows = nx.maximum_flow(self.graph, 's', 't')
        # print(max_flow)
        # for (frm, to), edge in self.graph.edges.items():
        #     print(f'{frm} -> {to}:', edge)
        if max_flow < self.matches * 2:
            return None
        
        return resulting_flows
