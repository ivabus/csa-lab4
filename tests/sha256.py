#!/usr/bin/env python3
import hashlib
import random

l = [random.randint(97, 122) for i in range(1, random.randint(1024, 102400))]


print(f'input = "{len(l)} {"".join(list(map(chr, l)))}"')

print(f'asserted_output = "{hashlib.sha256(bytes(l)).hexdigest()}"')
