import numba

@numba.jit
def zs2(n, U=None, M=None):
    x = [1] * (n+1)
    if M is None or n <= M:
        yield x[1:]
    x[0] = -1
    x[1] = 2
    h = 1
    m = n - 1
    if M is None or m <= M:
        yield x[1:][:m]

    while x[1] != n:
        if m - h > 1:
            h += 1
            x[h] = 2
            m -= 1
        else:
            j = m - 2
            while x[j] == x[m-1]:
                x[j] = 1
                j -= 1
            h = j + 1
            x[h] = x[m-1] + 1
            r = x[m] + x[m-1] * (m - h - 1)
            x[m] = 1
            if m - h > 1:
                x[m-1] = 1
            m = h + r - 1
        if U is not None and x[1] > U:
            return
        if M is not None and m > M:
            continue
        yield x[1:][:m]

def reduce_partition(p, N):
    m = N
    while m < len(p) and p[m] == p[m-1]:
        m += 1
    return tuple(p[:m])

def get_score_partitions(n_players, n_winners):
    n_matches = n_players * (n_players - 1)
    max_score = 2 * (n_players - 1)
    partitions = set(reduce_partition(p, n_winners) for p in zs2(n_matches, max_score, n_players))
    return sorted(partitions, key=lambda p: (len(p), -len(set(p)), reversor(p)))

class reversor:
    def __init__(self, obj):
        self.obj = obj

    def __eq__(self, other):
        return other.obj == self.obj

    def __lt__(self, other):
        return other.obj < self.obj