# Project Notes

## Meeting 2020-11-17

- Discussed what composing is
- A composition is at its core a set of rows along with a set of leads which start at those rows.
- Two main types of composition
  - ### <= 7 bells, i.e. 'Extents':
    
    This has two (usually distinct) phases
    - Figuring out what set of leads/courses form atomic fragments which build an extent
    - Figuring out how to order these extents in such a way that they can be rung sequentially

  - ### > 8 bells, i.e. Music-oriented composing:

    Because we don't have to cover every row, composers are essentially balancing music/simplicity.
    One possible workflow for this is to build a composition out of larger atomic blocks that contain
    desirable rows, and then later on stitching these blocks together.

- Noted that the second part of `<= 7` is very similar to `> 8`
- What we could do with is a 'companion' program to provide instant checks and feedback even on partial
  compositions

# Random musings on composition structure

## Permutation

A permutation represents a change from one row to another, but is not a sequence of bells.  For example,
the place notation `14` on Major corresponds to the permutation:
```
   1 2 3 4 5 6 7 8
-> 1 3 2 4 6 5 8 7
```

Note that permutations are not dependent on the row that they start:
```
   8 7 6 5 4 3 2 1
-> 8 6 7 5 3 4 1 2
```
and
```
   c o m p u t e r
-> c m o p t u r e
```
are equivalent definitions of the permutation represented by the `14` PN.

For convenience, I will write permutations as though they start from `123456...`, so the `14` PN
would become `<13246587>`.

## Block

A block is an ordered sequence of permutations (not rows).  A block has no defined starting row - it
can be modelled as a function that takes a start row and produces a sequence of rows:

```haskell
class Block a where
  length :: a -> Int
  rows :: a -> Row -> [Rows]
  leftover_row = last . rows
```

- A composition is an **unordered** set of fragments.
- A comp fragment is a sequence of chained blocks and a starting row.
- A row is a sequence of bells
- A bell is a newtyped integer

```haskell
type Comp = {Frag}
type Frag = (Row, Block)
type Block = Chain Block Block | SinglePermutation Perm
```

## More musings

- Permutations and rows are different things:
  - Permutations are functions of type `[T; n] -> [T; n]` and can permute anything
  - Row is a sequence of Bells.
  Permutations and rows are structurally similar - a row could be thought of as a permutation
  of rounds
- Can use `*` to compose a permutation onto a Row or permutation
- A block of changes is like a meta-permutation - takes a Row and turns it into a sequence of
  Rows which start with that given Row
  - The last Row is 'left over' - represents the Row after the Block ends

    e.g. a lead of Bastow starting at rounds:
    ```text
    123456
    214365
    213456
    124365
    ------
    142635
    ```

    Here 142635 is left over, and would be the start of the next block

  - A single permutation is a block that would generate only the row you gave it, and returns the
    that permutation applied to the given row as a leftover row
- Blocks can be appended to each other into a bigger block, where the second block would start with
  the left-over row of the first
- Blocks can be applied to a given Row to convert them into a fragment

## Facebook poll

- It seems that the building a comp from smaller blocks is quite a common thing that people want,
  even for people who mostly make extents.
- There was an impressive split between coursing orders and course heads
  - => There should be easy configuration between the two ways of viewing compositions
- Notable existing programs are e.g. 'Inpact' by Alexander Holroyd - good ideas about interactivity
  but is lacking features such as being able to start a block not from rounds.  Also Windows-only.
  Links: [main page](https://www.math.ubc.ca/~holroyd/inpact/help.html),
  [screenshot](https://www.math.ubc.ca/~holroyd/inpact/scrsht.gif),
  [download](https://www.math.ubc.ca/~holroyd/inpact/inpact1_2.zip)

## 2020-12-16

- Started writing code.  Implemented `Perm` and `Block` as generic permutations and blocks that can
  be used to permute any input.

## Chat w/ Alexander Holroyd

- Talked a lot about file formats for storing compositions; he mostly agreed with my thoughts
- He demonstrated Inpact, and it was cool.

  Pros of Inpact:
  - It is completely general; it makes no assumptions about the structure of your comp.  All comps
    are single blocks starting from rounds, represented as a DAG of nested blocks with blocks being
    able to be used multiple times:
    ```haskell
    data Comp = Comp Block
    data Block = Chain [Block] | Perm
    ```

    What we'd probably want is something more like this (where `{| ... |}` represents a bag, or an
    set with repeats):
    ```haskell
    data Comp = {| Frag |}
    data Frag = Frag Block Row
    data Block = Chain [Block] | Perm
    ```

    I do agree that generality is a worthy thing to strive for, but in order for an app to be
    useful I think it needs to have some pragmatic custom ways to display data (e.g. displaying
    comps as tenor-observation calling positions).
  - It is interactive - you right click to re-prove

  Cons of Inpact:
  - It can't handle 'partial' compositions
  - It is not particularly visual
  - The representation of falseness used is pretty unituitive - it simply tells you where the false
    rows are, not their relation
- I sketched out my ideas for how the program should look, and he generally approved:
  ![A-Holroyd sketch](a-holroyd.png)

## 2021-03-12

- Implemented muting/soloing in the traditional sense of having a mute/solo feature attached to each
  `Frag`.  But this is confusing, and FL Studio uses a better way.
- Noted that the architecture is basically client/server (JS/Rust), except that the client is in
  charge of storing all the data provided by the server (JS writing cookies)

## 2021-04-10: Chat w/ Mark B. Davies

- Generally big approval (yay!).
- Noted that non-group part heads are probably not useful enough to be a feature, since then
  inter-part `Frag` linking is very well-defined.  We do need some algorithm for computing the group
  generated by a set of rows, though.
- Also decided that allowing calls to change lead length is not useful enough to warrant the extra
  complexity.
- Highlight false rows red, or use numbers?  Perhaps make this a setting
- Decided that having a concept of 'stroke' is not well defined enough to implement
- Questioned whether or not serdeing to cookies is the right idea
