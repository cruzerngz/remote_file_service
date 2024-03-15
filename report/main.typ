// #set text(
//   font: "",
// )

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

#set heading(numbering: "1.1.")

// nice code formatting
#show raw.where(lang: "rs"): code => {
  block(
    fill: luma(247),
    radius: 3pt,
    outset: 5pt,
    stroke: gray,
    breakable: false
  )[#text(size: 0.85em)[#code]]
}

= Overview
This course project consists of the following requirements:
1. Implement a RPC protocol over UDP
2. Implement marshalling and unmarshalling of messages
3. Implement a request-reply protocol
4. Implement various invocation semantics (at-most-once, at-least-once, maybe)

The following additional requirements are also implemented:
1. In-flight data compression
2. Remote method code generation
3. Multiple UDP transmission protocols


// describe the overall design, ergonomics for a dev
= Design
At its core, this implementation provides various tools to abstract away the
complexity of defining and implementing a remote method invocation system.
From now on, this library will be referred to as `rfs`.


A remote interface can be defined as follows:
```rs
#[remote_interface]
pub trait SimpleOps {
    /// Pass something to the remote to log.
    async fn say_hello(content: String) -> bool;

    /// Compute the Nth fibonacci number and return the result.
    ///
    /// This is supposed to simulate an expensive computation.
    async fn compute_fib(fib_num: u8) -> u64;
}
```

The `#[remote_interface]` attribute is a procedural macro that modifies any arbitrary interface such that it can be invoked remotely.
The macro does the following:
- modify the original interface definition to include a mutable receiver (`&mut self`), for server-side persistence
  ```rs
  async fn say_hello(&mut self, content: String) -> bool {
    // .. implementation specific
  }
  ```

- generate a client-side proxy to remotely invoke such methods
  ```rs
  /// Method used by the client
  pub async fn say_hello<T: rfs_core::middleware::TransmissionProtocol>
    (
      ctx: &mut rfs_core::middleware::ContextManager<T>,
      content: String,
    ) -> Result<bool, rfs_core::middleware::InvokeError>;
  ```

- a request-reply data type used for this interface method
  ```rs
  /// SimpleOps::say_hello generates this method payload.
  pub enum SimpleOpsSayHello {
      Request { content: String },
      Response(bool),
  }
  ```

- various other interfaces for use with the `ContextManager`

#pagebreak()

// describe the ser/de process and the implementation
== Message format

// describe the simple data compression logic for this particular message format
=== Data Compression

// describe the use of macros for generating/deriving middleware and other code
== Code generation

// describe the middleware logic
== Middleware

// not sure what this is here for
== Fault tolerance

=== Maybe invocation semantics
=== At-least-once invocation semantics
=== At-most-once invocation semantics

// report on the results of the experiments:
// - at-most-once invocation semantics
// - at-least-once invocation semantics
//
// what's expected:
// at-least-once can lead to wrong results for non-idempotent operations
// at-most-once work correctly for all operations
= Experiments

