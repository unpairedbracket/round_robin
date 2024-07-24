from argparse import ArgumentParser
from collections import Counter
import itertools
import math
import tomllib

import numpy as np
from tqdm import tqdm

import matplotlib.pyplot as plt

from results import Results
from partition import get_score_partitions

def analyse_state(current_results: Results, competitors: list[str], n_winners=3):
    n_competitors = len(competitors)
    competitor_ids = list(range(n_competitors))
    all_partitions = get_score_partitions(n_competitors, n_winners)
    current_scores = current_results.scores()
    max_scores = current_results.max_possible_scores()
    possible_results = {}
    for top_guys in tqdm(itertools.permutations(competitor_ids, n_winners), total=math.perm(n_competitors, n_winners)): # perm
        scores_table = check_top_permutation(current_results, top_guys, competitor_ids, all_partitions, current_scores, max_scores)
        if scores_table is not None:
            possible_results[top_guys] = scores_table
        # if table is None:
        #     pass
        #     print(f"{[competitors[guy] for guy in top_guys]} is impossible")
        # else:
        #     print(f"{[competitors[guy] for guy in top_guys]} is possible:")
        #     scores = table.scores()
        #     inds = np.argsort(-scores)
        #     for i in inds:
        #         print(f"{competitors[i]:>20}: {scores[i]:02d}")
    return possible_results

def analyse_state_0(current_results: Results, competitors: list[str], n_winners=3):
    n_competitors = len(competitors)
    competitor_ids = list(range(n_competitors))
    current_scores = current_results.scores()
    max_scores = current_results.max_possible_scores()
    possible_results = {}
    
    top_guys = (6,7,0)

    others = [cid for cid in competitor_ids if cid not in top_guys]
    required_scores = (8, 8, 8)

    scores_table = check_score_partition(current_results, required_scores, top_guys, current_scores, max_scores, others)

    possible_results[top_guys] = scores_table
    if scores_table is None:
        pass
        print(f"{[competitors[guy] for guy in top_guys]} is impossible")
    else:
        target_scores, table = scores_table
        scores = table.scores()
        scores_mod = scores.copy()
        for k, guy in enumerate(reversed(top_guys)):
            scores_mod[guy] += 100 * (k+1)
        inds = np.argsort(-scores_mod)
        for i in inds:
            print(f"{competitors[i]:>20}: {scores[i]:02d}")
        print(f"{'':>16}  " + " ".join([c[0] for c in competitors]))
        for c, row in enumerate(table.result_array):
            print(f'{competitors[c]:>16}: ' + ' '.join(row))

        

def check_top_permutation(current_results: Results, top_guys, competitor_ids, all_partitions, current_scores, max_scores):
    others = [cid for cid in competitor_ids if cid not in top_guys]
    for required_scores in (all_partitions):
        scores_table = check_score_partition(current_results, required_scores, top_guys, current_scores, max_scores, others)
        if scores_table is not None:
            return scores_table



def check_score_partition(current_results: Results, required_scores, top_guys, current_scores, max_scores, others):
    for player, req in zip(top_guys, required_scores):
        if (current_scores[player] > req) or (max_scores[player] < req):
            return None
        
    if sum(required_scores) > (current_results.scores() + current_results.number_remaining()).sum():
        print("Not enough matches left to satisfy the required scores")
        return None

    n_guys_total = len(required_scores)
    n_winners = len(top_guys)
    number_others_needed = n_guys_total - n_winners
    for other_guys in itertools.combinations(others, number_others_needed):
        modified_results = current_results.clone()
        scores_table = check_with_guys(modified_results, required_scores, top_guys, other_guys)
        if scores_table is not None:
            return scores_table

                

def check_with_guys(results: Results, required_scores, top_guys, other_guys):
    all_guys = top_guys + other_guys
    n_winners = len(top_guys)
    n_guys_total = len(all_guys)
    n_competitors = results.N
    
    # Set up necessary wins to avoid condorcet cycles
    # within sets of people with the same number of points 
    for j in range(n_winners):
        # print(f"j={j}")
        for k in range(j+1, n_guys_total):
            # print(f'k={k}')
            if required_scores[k] < required_scores[j]:
                # We're out of the drawn set for competitor j now
                break
            if required_scores[k] == required_scores[j]:
                cj = all_guys[j]
                ck = all_guys[k]
                # j and k tie on final score so j must beat k
                match results.result_array[cj, ck]:
                    case 'w':
                        # print(f"{cj} already beats {ck}")
                        pass
                    case 'o':
                        # print(f"{cj} hasn't fought {ck} yet, setting to win")
                        results.win(cj, ck)
                    case 'l':
                        # print(f"{cj} lost to {ck}, aborting")
                        return None
                    case 'd':
                        # print(f"{cj} drew against {ck}, aborting")
                        return None
                    case 'x':
                        print(f"error: {cj} and {ck} seem to be the same person??")
                        return None
                    case 'z':
                        # print(f"{cj} had a no-contest with {ck}, aborting")
                        return None
                    case other:
                        print(f"unknown match state {other}")
                        return None
            else:
                print("scores shouldn't be able to go up??")

    modified_scores = results.scores()
    modified_max = results.max_possible_scores()
    lower_bounds = modified_scores.copy()
    upper_bounds = np.ones(n_competitors) * (required_scores[-1] - 1)

    # now the draws are settled, check we can satisfy the required scores
    for player, req in zip(all_guys, required_scores):
        # no slack for these guys, you must have _exactly_ the required score
        lower_bounds[player] = req
        upper_bounds[player] = req

    # print(lower_bounds, modified_scores, modified_max, upper_bounds)

    if (lower_bounds > upper_bounds).any():
        return None
    if (modified_scores > upper_bounds).any():
        return None
    if (modified_max < lower_bounds).any():
        return None
    if (lower_bounds - modified_scores).sum() > results.number_remaining().sum():
        return None
    
    # we can finally make the graph now
    g = results.to_max_flow_graph(lower_bounds, upper_bounds)
    if g is None:
        print("can't construct graph?")
        return None
    if (flows := g.can_be_satisfied()) is not None:
        # success!
        return required_scores, results.with_resolution(flows)
            

def print_competitor(name, score, max_score, comment=""):
    print("")
    print("")
    print(f"{name:>20}:  {int(score):2d}  (Max final score: {int(max_score):2d}){comment}")

def print_runnerup(name):
    print(f"{'':28}possible runner-up: {name}")

def print_result(winner, runnerup, competitors, satisfying_results):
    if runnerup is None:
        runnerup_name = "anyone"
    else:
        runnerup_name = competitors[runnerup]
    print(f"{'':32}Winner: {competitors[winner]}, Runner-up: {runnerup_name}")
    for c, score in sorted(enumerate(satisfying_results.scores()), key=lambda x: -x[1]):
        print(f"{'':32}{competitors[c]:>20}: {int(score):2d}")

def main():
    plt.rcParams['xtick.bottom'] = plt.rcParams['xtick.labelbottom'] = False
    plt.rcParams['xtick.top'] = plt.rcParams['xtick.labeltop'] = True
    pars = ArgumentParser()
    pars.add_argument("result_file")

    args = pars.parse_args()


    with open(args.result_file, 'rb') as f:
        g1 = tomllib.load(f)

    n_winners = g1['number_advance']
    for block in g1['block']:
        competitors = block['competitors']
        n_competitors = len(competitors)

        r = Results(n_competitors)
        for night_number, night in enumerate(block['nights']):
            r.apply_results(**night)

            print(f"After night {night_number+1}:")
            possible_results = analyse_state(r, block['competitors'], n_winners)

            result_matrix = np.zeros(n_winners * (n_competitors,)) + 3.5
            for result, (scores, table) in possible_results.items():
                biggest_draw = max(Counter(scores).values())
                if biggest_draw == 1:
                    result_matrix[result] = 2.5
                elif biggest_draw == 2:
                    result_matrix[result] = 0.5
                elif biggest_draw > 2:
                    result_matrix[result] = 1.5
            if n_winners == 2:
                for n in range(n_competitors):
                    result_matrix[n,n] = np.nan
                plt.imshow(result_matrix, extent=(0, n_competitors, 0, n_competitors), cmap='tab10', vmin=0, vmax=10)
            if n_winners == 3:
                for n in range(n_competitors):
                    result_matrix[:,n,n] = np.nan
                    result_matrix[n,:,n] = np.nan
                    result_matrix[n,n,:] = np.nan
                image = result_matrix.swapaxes(1,2).reshape(-1, n_competitors)
                plt.imshow(image, extent=(0, n_competitors, 0, n_competitors), cmap='tab10', vmin=0, vmax=10)
                x, y = np.mgrid[:(1+n_competitors), :(1+n_competitors)]
                plt.plot(x, y, 'k', x.T, y.T, 'k')
                for n, c in enumerate(competitors):
                    for j in range(n_competitors):
                        for i in range(n_competitors):
                            plt.text(j + 0.05, i + 1 - (n + 0.5) / n_competitors, c, color='w', va='center_baseline', fontsize='xx-small')
            xtick_positions = np.arange(n_competitors) + 0.5
            ytick_positions = np.arange(n_competitors)[::-1] + 0.5
            plt.plot([],[], color=plt.cm.tab10(0.25), label="Possible without draws", linewidth=5)
            plt.plot([],[], color=plt.cm.tab10(0.05), label="Possible with 2-way draw", linewidth=5)
            plt.plot([],[], color=plt.cm.tab10(0.15), label="Possible with n-way draw", linewidth=5)
            plt.plot([],[], color=plt.cm.tab10(0.35), label="Impossible", linewidth=5)
            plt.legend(loc='center left', bbox_to_anchor=(1, 0.5))
            plt.xticks(xtick_positions, [''.join(n[0] for n in c.split(' ')) for c in competitors])
            plt.yticks(ytick_positions, competitors)
            plt.title(f"Block {block['name']}, after night {1 + night_number}")
            plt.show()
            # input()
            # break

if __name__ == '__main__':
    main()