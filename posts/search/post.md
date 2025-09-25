### Search is about choice, not map
### Option

|                                  | Backtracking | Use Enqueued List | Informed |
| -------------------------------- | ------------ | ----------------- | -------- |
| **British Museum**               | X            | X                 | X        |
| **DFS**                          | ✔️           | ✔️                | X        |
| **BFS**                          | X            | ✔️                | X        |
| **Hill Climbing** (Improved DFS) | ✔️           | ✔️                | ✔️       |
| **Beam** (Improved BFS)          | X            | ✔️                | ✔️       |
### Enqueue
#### Example Queue
![Image](/static/img/posts/search/Pasted%20image%2020250414170213.webp)
From **S** to **G** (Using DFS)
~~(S)~~
~~(S A)~~ (S B)
~~(S A B)~~ (S A D) (S B)
~~(S A B C)~~ (S A D) (S B)
~~(S A B C E)~~ (S A D) (S B)
~~(S A D)~~ (S B)
(S A D G) (S B)
### How to Enqueue
**DFS:** Front

**BFS:** Back
**Hill Climbing:** Front Sorted
**Beam:** Keep *`W`* Best

**Hill Climbing** and **Beam**: Consider the distance to the goal

Consider how far we've gone so far
1. Initialize the queue
2. Test first path in the queue


![Image](/static/img/posts/search/Pasted%20image%2020250414182611.webp)
####  Admissible Heuristic
The heuristic estimate is guaranteed to be less than the actual distance

It is a perfectly sound way of doing an optimal search when about a map, but it may not work when it's not about a map.
So we need **Consistency**.

#### Consistent Heuristic
