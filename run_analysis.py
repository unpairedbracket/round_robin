from argparse import ArgumentParser
import itertools
import tomllib

# import numpy as np

from results import Results

def analyse_state(current_results: Results, competitors: list[str]):
    current_scores = current_results.scores()
    possible_lineups = {}
    for winner, (winner_name, score, max_final) in enumerate(zip(competitors, current_scores, current_results.max_possible_scores())):
        # Figure out if `winner` can come unambiguously first
        scores_except_current = current_scores.copy()
        scores_except_current[winner] = 0
        
        if max_final < scores_except_current.max():
            # There is some other competitor who already has as many
            # or more points than our candidate can possibly get
            print_competitor(winner_name, score, max_final, " (trivially eliminated from first-place contention)")
        else:
            candidate_wins_all = current_results.clone()
            for loser in candidate_wins_all.remaining_matches_for_competitor(winner):
                candidate_wins_all.win(winner, loser)
            new_scores = candidate_wins_all.scores()
            assert new_scores[winner] == max_final
            if g := candidate_wins_all.to_max_flow_graph(new_scores, max_final - 1, (winner,)):
                flows = g.can_be_satisfied()
                if flows is not None:
                    # We can satisfy all the conditions for this person to come first without draws!
                    print_competitor(winner_name, score, max_final)
                    satisfying_results = candidate_wins_all.with_resolution(flows)
                    print_result(winner, None, competitors, satisfying_results)
                    print(satisfying_results.result_array)

                    for runnerup, (runnerup_name, max_final2nd) in enumerate(zip(competitors, candidate_wins_all.max_possible_scores())):
                        if runnerup == winner:
                            # Can't come second if you're also first
                            continue
                        # print(f"{'':20}Trying to find solution for {runnerup_name} in second place")
                        second_layer_results = candidate_wins_all.clone()
                        # Try and find a solution with winner > runnerup > everyone else (no draws)
                        if max_final2nd < max_final:
                            # print('option A')
                            # Just let the second-place pick win all of their matches now
                            for loser in second_layer_results.remaining_matches_for_competitor(runnerup):
                                second_layer_results.win(runnerup, loser)
                            new_scores = second_layer_results.scores()
                            assert new_scores[runnerup] == max_final2nd                            

                            if g := second_layer_results.to_max_flow_graph(new_scores, max_final2nd - 1, (winner, runnerup)) :
                                flows = g.can_be_satisfied()
                                if flows is not None:
                                    # We can satisfy all the conditions for this person to come second!
                                    satisfying_results = second_layer_results.with_resolution(flows)
                                    print_runnerup(runnerup_name)
                                    print_result(winner, runnerup, competitors, satisfying_results)
                                    possible_lineups[(winner, runnerup)] = satisfying_results
                                    continue
                            if current_results.result_array[winner, runnerup] == 'o':
                                # We can adjust the result of winner vs. runnerup
                                if max_final2nd < max_final - 2:
                                    second_layer_results.draw(winner, runnerup)
                                    new_scores = second_layer_results.scores()
                                    assert new_scores[winner] > new_scores[runnerup]
                                    if g := second_layer_results.to_max_flow_graph(new_scores, new_scores[runnerup] - 1, (winner, runnerup)) :
                                        flows = g.can_be_satisfied()
                                        if flows is not None:
                                            satisfying_results = second_layer_results.with_resolution(flows)
                                            # We can satisfy all the conditions for this person to come second!
                                            print_runnerup(runnerup_name)
                                            print_result(winner, runnerup, competitors, satisfying_results)
                                            possible_lineups[(winner, runnerup)] = satisfying_results
                                            continue
                                if max_final2nd < max_final - 4:
                                    second_layer_results.win(runnerup, winner)
                                    new_scores = second_layer_results.scores()
                                    assert new_scores[winner] > new_scores[runnerup]
                                    if g := second_layer_results.to_max_flow_graph(new_scores, new_scores[runnerup] - 1, (winner, runnerup)) :
                                        flows = g.can_be_satisfied()
                                        if flows is not None:
                                            satisfying_results = second_layer_results.with_resolution(flows)
                                            # We can satisfy all the conditions for this person to come second!
                                            print_runnerup(runnerup_name)
                                            print_result(winner, runnerup, competitors, satisfying_results)
                                            possible_lineups[(winner, runnerup)] = satisfying_results
                                            continue    
                        else:
                            # print("option B")

                            # Find a solution where runnerup has one less point than winner
                            new_scores = second_layer_results.scores()
                            lower_limit = new_scores.copy()
                            upper_limit = new_scores.copy()
                            
                            # Not sure if this is necessary?
                            # I feel like a gap of 1 should be sufficient but I can't prove it
                            for gap in range(1, max_final):
                                # print(f"gap = {gap}")
                                upper_limit[:] = max_final - gap - 1
                                lower_limit[runnerup] = max_final - gap
                                upper_limit[runnerup] = max_final - 1
                                lower_limit[winner] = max_final
                                upper_limit[winner] = max_final
                                if g := second_layer_results.to_max_flow_graph(lower_limit, upper_limit, (winner, )):
                                    # print("got graph")
                                    flows = g.can_be_satisfied()
                                    if flows is not None:
                                        satisfying_results = second_layer_results.with_resolution(flows)
                                        # We can satisfy all the conditions for this person to come second!                                    
                                        print_runnerup(runnerup_name)
                                        print_result(winner, runnerup, competitors, satisfying_results)
                                        possible_lineups[(winner, runnerup)] = satisfying_results
                                        break
                                    # else:
                                        # print("Can't compute satisfying flow")
                                        # print(second_layer_results.result_array)
                                        # print(second_layer_results.scores(), 'scores')
                                        # print(lower_limit, 'lower limits')
                                        # print(upper_limit, 'upper limits')

                                # else:
                                    # print("Can't construct graph")
                            if (winner, runnerup) in possible_lineups:
                                continue
                            # else:
                            #     print("failed")
                            #     g = candidate_wins_all.to_max_flow_graph(candidate_wins_all.scores(), max_final - 1, (winner,))
                            #     flows = g.can_be_satisfied()
                            #     satisfying_results = candidate_wins_all.with_resolution(flows)
                            #     print_result(winner, runnerup, competitors, satisfying_results)


                        print(f"{'':28}can't come second without draws: {runnerup_name}")
                        # So let's try to find some second-places with draws
                        # Here we know winner can win without any draws, so only consider n-way draws for second place
                        other_competitor_ids = set(range(len(competitors))) - {winner, runnerup}
                        for n_others in range(len(other_competitor_ids)):
                            for also_drawn in itertools.combinations(other_competitor_ids, n_others):
                                draw_results = current_results.clone()
                                for c in also_drawn:
                                    draw_results.win(runnerup, c)
                                new_scores3 = draw_results.scores()
                                max_scores3 = draw_results.max_possible_scores()
                                drawn_set = [runnerup] + list(also_drawn)
                                max_possible_draw = min(new_scores3[winner] - 1, max_scores3[drawn_set].min())
                                min_possible_draw = new_scores[drawn_set].max()
                                for target_score in range(max_possible_draw, min_possible_draw-1, -1):
                                    lower_limit = new_scores3.copy()
                                    upper_limit = max_scores3.copy()
                                    lower_limit[winner] = target_score + 1
                                    lower_limit[drawn_set] = target_score
                                    upper_limit[:] = target_score - 1
                                    upper_limit[drawn_set] = target_score
                                    upper_limit[winner] = max_scores3[winner]
                                    if g := draw_results.to_max_flow_graph(lower_limit, upper_limit, (winner, )):
                                        # print("got graph")
                                        flows = g.can_be_satisfied()
                                        if flows is not None:
                                            satisfying_results = second_layer_results.with_resolution(flows)
                                            # We can satisfy all the conditions for this person to come second!                                    
                                            print_runnerup(runnerup_name)
                                            print_result(winner, runnerup, competitors, satisfying_results)
                                            possible_lineups[(winner, runnerup)] = satisfying_results
                                            break

                                    





                else:
                    print_competitor(winner_name, score, max_final, " (mathematically eliminated: can't compute winning flow)")
            else:
                print_competitor(winner_name, score, max_final, " (mathematically eliminated: can't construct flow graph)")
            

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
    pars = ArgumentParser()
    pars.add_argument("result_file")

    args = pars.parse_args()


    with open(args.result_file, 'rb') as f:
        g1 = tomllib.load(f)


    for block in g1['block']:
        competitors = block['competitors']
        n_competitors = len(competitors)

        r = Results(n_competitors)
        for night_number, night in enumerate(block['nights']):
            r.apply_results(**night)
            print(f"After night {night_number+1}:")

            analyse_state(r, block['competitors'])
            # return

if __name__ == '__main__':
    main()