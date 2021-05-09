# xpub-balance
Checks the balance of an xpub and its addresses.
Uses Esplora to collect address informations.

Usage
------------
```
xpub-balance 0.1.0
Checks the balance of an xpub and its addresses

USAGE:
    xpub-balance.exe [FLAGS] [OPTIONS] <xpub> [ARGS]

FLAGS:
    -c, --change     Show change instead of receive addresses. Doesn't effect the total balance calculation
    -h, --help       Prints help information
    -o, --offline    Only show addresses. No reqests will be send
    -V, --version    Prints version information

OPTIONS:
    -e, --esplora <esplora>    Use a specific Esplora URL [default: https://blockstream.info/api/]
    -n <n>                     Total number of indexes to check. Each index has two addresses: a receive and a change
                               address. Relevant for the total balance calculation [default: 100]

ARGS:
    <xpub>     Extended public key of your wallet account. Either xpub, ypub or zpub
    <start>    First index to print [default: 0]
    <end>      Last index to print [default: 15]
```

Example
------------
```
$xpub-balance xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brx
MyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj  0 10

0/0   1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA    0 sat  12 txs
0/1   1Ak8PffB2meyfYnbXZR9EGfLfFZVpzJvQP    0 sat  4 txs 
0/2   1MNF5RSaabFwcbtJirJwKnDytsXXEsVsNb    0 sat  6 txs
0/3   1MVGa13XFvvpKGZdX389iU8b3qwtmAyrsJ    0 sat  2 txs
0/4   1Gka4JdwhLxRwXaC6oLNH4YuEogeeSwqW7    0 sat  2 txs
0/5   19a7HGg32ecPQo49rDeM2NSFJHPqrwSJto    0 sat  9 txs
0/6   1GuMEkKyqqRz3jKZJPNxZNoJv72rRDm88o    0 sat  7 txs
0/7   1B1wDxGPrfqWSi4qvQvaPdunD6kon3CeDG    0 sat  4 txs
0/8   1BMZTqDtNogSEs1oZoGxRqfR6jS2tVxvHX    0 sat  4 txs
0/9   1DUrqK4hj6vNNUTWXADpbqyjVWUYFD7xTZ    0 sat  2 txs
0/10  146emAmGumhnsT9nPCALU2JWeS4koxfFRB    0 sat  2 txs

-> total balance     : 0 sat
-> total transactions: 77 txs
```
