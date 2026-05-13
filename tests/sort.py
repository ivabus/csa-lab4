#!/usr/bin/env python3

import random

l = [i for i in range(1, 100)]

print(f'asserted_output = "{", ".join(map(str, l))}\\n"', sep=", ")
random.shuffle(l)
print(f'input = "{" ".join(map(str, l))} 0"')
