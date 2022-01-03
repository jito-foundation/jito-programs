# JITO Payment Program
This program acts as the interface for Leaders to be paid to include searcher bundles.

The workflow is as follows:
1. Searcher calls the initialize instruction
and the last transaction is a payment to the leader (both of these txs are function calls invoked on this program)
1. Valdators will filter out bundles that do not include these two txs (shipped as part of the jito-solana fork)
1. Validator simulates the bundle and decides whether or not to include in block (i.e. was validator payment favorable?)
1. Validator includes tx(s) to sweep funds before submitting the new block to the network
