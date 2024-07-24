import numpy as np

from graph import Graph


class Results:
    def __init__(self, n_competitors):
        self.N = n_competitors
        self.result_array = np.array(
            [
                ["x" if j == k else "o" for j in range(n_competitors)]
                for k in range(n_competitors)
            ]
        )

    def scores(self):
        return (2 * (self.result_array == "w") + 1 * (self.result_array == "d")).sum(
            axis=1
        ).astype('int')

    def max_possible_scores(self):
        return (self.scores() + 2 * self.number_remaining()).astype('int')

    def win(self, winner, loser):
        self.result_array[winner, loser] = "w"
        self.result_array[loser, winner] = "l"

    def draw(self, drawer_a, drawer_b):
        self.result_array[drawer_a, drawer_b] = "d"
        self.result_array[drawer_b, drawer_a] = "d"

    def remaining_matches(self):
        remaining = []
        for j in range(self.N):
            for k in range(j + 1, self.N):
                if self.result_array[j, k] == "o":
                    remaining.append((j, k))
        return remaining
    
    def remaining_matches_for_competitor(self, competitor):
        return [k for k in range(self.N) if self.result_array[competitor, k] == "o"]

    def number_remaining(self):
        return (self.result_array == "o").sum(axis=1)

    def clone(self):
        r = Results(self.N)
        r.result_array = self.result_array.copy()
        return r

    def to_max_flow_graph(self, lower_score_bounds, upper_score_bounds, fixed_competitors=()) -> Graph | None:
        g = Graph()
        g.add_node('pre-t')
        scores = self.scores()

        lower_score_bounds = np.fmax(scores, lower_score_bounds)
        
        min_needed = lower_score_bounds - scores # demand
        max_needed = upper_score_bounds - scores # capacity

        slack = max_needed - min_needed

        matches_left = self.remaining_matches()

        pre_t_capacity = 2 * len(matches_left)
        for c, remaining in enumerate(self.number_remaining()):
            if c in fixed_competitors:
                assert remaining == 0
                continue
            if max_needed[c] < 0:
                print(f"{c} already has too many points")
                return None
            if min_needed[c] >  2 * remaining:
                print(f"{c} cannot attain enough points")
                return None
            if slack[c] < 0:
                print(f"{c} has min score more than max score")
                return None
                
            g.add_node(c, f'c{c}')

            pre_t_capacity -= min_needed[c]
            if min_needed[c] > 0:
                g.add_edge(c, 't', min_needed[c])
            if slack[c] > 0:
                g.add_edge(c, 'pre-t', slack[c])

        for (a, b) in matches_left:
            g.add_node((a,b), f'{a}|{b}')
            g.add_edge((a,b), a, None)
            g.add_edge((a,b), b, None)
            g.add_edge('s', (a,b), 2)

        if pre_t_capacity > 0:
            g.add_edge('pre-t', 't', pre_t_capacity)
        elif pre_t_capacity < 0:
            print("pre-t capacity negative")
            return None


        return g
    
    def apply_results(self, wins=[], draws=[]):
        for (w, l) in wins:
            self.win(w, l)
        for (d0, d1) in draws:
            self.draw(d0, d1)

    def with_resolution(self, flows):
        resolved = self.clone()
        for node, outgoing_flows in flows.items():
            match node:
                case (j, k):
                    result = (outgoing_flows[j], outgoing_flows[k])
                    if result == (2, 0):
                        resolved.win(j, k)
                    elif result == (0, 2):
                        resolved.win(k, j)
                    elif result == (1, 1):
                        resolved.draw(k, j)
                case _:
                    pass
        assert resolved.number_remaining().sum() == 0
        return resolved