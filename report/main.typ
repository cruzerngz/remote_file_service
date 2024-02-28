// cover page
#align(center, text(size: 1.5em, weight: "bold")[

  #image("media/ntu_logo.svg", width: 75%)

  CE4013 Distributed Systems
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  2023/2024 Semester 2 course project:

  _Design and Implmentation of A System for Remote File Access_
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  Ng Jia Rui: U2020777D (100%)
  #linebreak()
  #linebreak()
  SCHOOL OF COMPUTER SCIENCE AND ENGINEERING
  NANYANG TECHNOLOGICAL UNIVERSITY
  #pagebreak()
])

#set heading(numbering: "1.1")

= Overview

= Design

// describe the ser/de process and the implementation
== Message format

// describe the simple data compression logic for this particular message format
=== Data Compression

// describe the use of macros for generating/deriving middleware and other code
== Code generation

// describe the middleware logic
== Middleware

// not sure what this is here for
= Implementation

// report on the results of the experiments:
// - at-most-once invocation semantics
// - at-least-once invocation semantics
//
// what's expected:
// at-least-once can lead to wrong results for non-idempotent operations
// at-most-once work correctly for all operations
= Experiments

