#set text(
  font: "Nimbus Roman",
)

// nice code formatting
#show raw.where(lang: "rs"): code => {
  block(
    fill: luma(247),
    radius: 3pt,
    outset: 5pt,
    stroke: gray,
    breakable: false,
    width: 100%, // does this look nice?
  )[#text(size: 0.8em)[#code]]
}

// pseudo code formatting
#show raw.where(lang: "py"): code => {
  block(
    fill: rgb("#FFEDEF"),
    radius: 3pt,
    outset: 5pt,
    stroke: red,
    breakable: false,
    width: 100%, // does this look nice?
  )[#text(size: 0.8em)[#code]]
}

// table style - I want to emulate the look seen in papers
#set table(
  stroke: (x, y) => (
    y: if y <= 1 {1pt} else {0.1pt},
    left: 0pt,
    right: 0pt,
    bottom: 1pt,
  ),
)

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

#set page(numbering: "1", paper: "a4")
#set heading(numbering: "1.1.")

// outline
#outline(indent: true,)
#linebreak()
#outline(title: "Tables", target: figure.where(kind: table))
#linebreak()
#outline(title: "Figures", target: figure.where(kind: image))
#pagebreak()

= Overview
This course project consists of the following requirements:
1. Implement a RPC protocol over UDP
2. Implement marshalling and unmarshalling of messages
3. Implement a request-reply protocol
4. Implement various invocation semantics (at-most-once, at-least-once, maybe)

The following additional requirements are also implemented:
1. In-flight data compression
2. Remote method code generation
3. Pluggable UDP transmission protocols with various levels of fault tolerance
4. Client terminal user interface

#linebreak()
// describe the overall design, ergonomics for a dev
= Design
At its core, this implementation provides various tools to abstract away the
complexity of defining and implementing a remote method invocation system.

From now on, this library will be referred to as `rfs`.

#linebreak()
#figure(
    image(
        "media/overview.svg",
        width: 65%
    ),
    caption: [Overview of `rfs`],
)

#pagebreak()
== Definition
#linebreak()
#figure(
    image(
        "media/interface_gen.svg",
        width: 95%
    ),
    caption: [An interface definition using `rfs`],
)

An interface can be defined with `rfs` like the one below.
Remote interfaces will serve as the foundation for all other abstractions (remote file objects, callbacks, etc.).

// #figure(caption: [asdadas], supplement: [Code block])[#align(left)[]]
```rs
#[remote_interface]
pub trait SimpleOps {
    /// Pass something to the remote to log.
    async fn say_hello(content: String) -> bool;

    /// Simulate an expensive computation.
    async fn compute_fib(fib_num: u8) -> u64;
}
```

The `#[remote_interface]` attribute is a procedural macro that modifies any arbitrary interface such that it can be invoked remotely.
The macro does the following:
- Modify the original interface definition to include a mutable receiver (`&mut self`), for optional persistence. The implementer does not need to take into account any network-related logic.
    ```rs
    async fn say_hello(&mut self, content: String) -> bool { /* implementation specific */ }
    ```

- Generate a client-side proxy to remotely invoke such methods
    ```rs
    /// Method used by the client
    /// Function has a new ContextManager parameter
    pub async fn say_hello<T: TransmissionProtocol> // misc trait bounds excluded
        (
        ctx: &mut ContextManager<T>,
        content: String,
        ) -> Result<bool, InvokeError>;
    ```

- Generate a request-reply data type used for this interface method

- Implement various other interfaces for use with the `ContextManager`


// describe the ser/de process and the implementation
== Message format
Each remote method has its associated enum with request and response variants.
The request variants contain the method arguments, while the response variants contain the return value.
The entire enum, along with it's method signature, is serialized when sent over the network.
```rs
/// SimpleOps::say_hello generates this method payload.
/// The method signature is also it's path: "SimpleOps::say_hello"
pub enum SimpleOpsSayHello {
    Request { content: String },
    Response(bool),
}
```

#pagebreak()
=== Marshalling/Unmarshalling
The terms "marshalling" and "unmarshalling" will be used interchangeably with "serialization" and "deserialization" throughout this report.

With #link("https://serde.rs/")[`serde`] providing helper macros, a custom data format is defined
with the ability to marshall and unmarshall arbitrary data types.
This brings a lot of flexibility when defining custom payloads.
```rs
/// serde provides the necessary macros to call the correct marshall/unmarshall functions
/// for any data type.
///
/// It is up to the implementer to define a serialization format for each data type.
///
/// This is an enum containing a C-style variant, a newtype variant, and a
/// struct variant.
/// Tuples, arrays and maps are also supported.
#[derive(Serialize, Deserialize)]
enum CustomPayload {
    Empty,
    Small(u64),
    Large {
        message: (SystemTime, String),
        data: Vec<u8>,
        lookup: HashMap<String, u32>,
    }
}
```

==== Design
There are some design considerations made for this serialization format.

- As a binary (byte-level) format, there is no guarantee that serialized data is human-readable.
- All multi-byte primitives such as numerics and floats are serialized in big-endian, or Network Byte Order (NBO).
- The atomic unit of data used throughout the marshalling process is a byte.
- The data format is partially self-describing. As implementing a self-describing format is not part of the requirements of the project, some data types share the same serialization method.
- Various byte prefixes and delimiters are used to assert the type of the data during unmarshalling. These bytes are referenced by their equivalent ASCII character. @serde_format_table describes the format with more detail.

#figure(
    caption: [Marshalling format for common rust data types],
    table(
        align: left,
        columns: (auto, auto, auto),
            [*data type*], [*byte prefix*], [*format*],
            [`boolean`], [`c`], [
                `true` $=>$ `u8::MAX`, `false` $=>$ `u8::MIN`

                *Example:* `c[0b1111_1111]`, `c[0b0000_0000]`
            ],
            [`char`], [none], [Serialized as UTF-8 (4 bytes), NBO],
            [`numeric`], [`n`], [Cast to `u64` and serialized in 8 bytes, NBO],
            [`float`], [`f`], [Cast to `f64` and serialized in 8 bytes, NBO],
            [`array`], [`s`], [
                Serialized as a length-prefixed array.
                Array bounds are delimited by square brackets (`[`, `]`).

            *Example:* `s[[ARR_LENGTH][ITEM1][ITEM2][...]]`],
            [`tuple`], [`t`], [Same as `array`, bounds are delimited by parantheses (`(`, `)`)],
            [`bytes`], [`b`], [
                Same as `array`
            ],
            [`string`], [`s`], [Same as `bytes`],

            [`struct`], [`m`], [
                Field-values are serialized as key-value pairs. Field names are serialized as strings. Field-value pairs are enclosed in angle brackets (`<`, `>`) and delimited by a hyphen (`-`).
                Boundaries are delimited by curly brackets (`{`, `}`).

                *Example:* `m{<[FIELD_NAME]-[VALUE]><[FIELD_NAME_2]-[VALUE_2]>}`],

            [`map`], [`m`], [same as `struct`],

            [`enum`], [`e`], [
                Variant names are prefixed before the inner value as strings.
                The inner value is serialized according to its respective data type.

                *Example:* `e[VARIANT_NAME][VARIANT_VALUE]`],

            [`option`], [`o`], [
                `Some<T>` is encoded as `u8::MAX`, while `None` is encoded as `u8::MIN`.
                The inner value is serialized according to it's respective data type.

                *Example:* `o[0b1111_1111][OPTION_VALUE]`, `o[0b0000_0000]`],

            // idk what else to add
    )
) <serde_format_table>

#pagebreak()

// describe the simple data compression logic for this particular message format
=== Data Compression
Due to the way numeric types are serialized, there are opportunities to compress the data.
Numeric types that are not 64-bit are cast to 64-bit before serializing.
Numeric types are used to prefix the length of arrays and strings.

This means that `0` bytes can take up a large proportion of data in a serialized payload.

A simple compression algorithm is implemented to reduce the footprint of the serialized data.
For byte arrays, each element is serialized directly to its corresponding byte value without casting from 8 to 64 bits.
This circumvents the need to perform redundant compression.
```py
# pseudocode
def compress(data: bytes) -> bytes:
    compressed = []

    while not data.end():
        # find the next zero byte
        non_zero_bytes = find_next_zero(data)
        compressed.append(non_zero_bytes)

        # count the number of consecutive zero bytes
        num_zeros = count_zeros(data)

        match num_zeroes:
            # a compressed sequence is 3 bytes large
            1:3 => compressed.append(number_of_zeroes(num_zeroes))

            4:255 => compressed.append([BYTE_COUNT_DELIM, num_zeroes, BYTE_COUNT_DELIM])

            # sequences of >256 bytes are compressed in 255-byte chunks
            256: => compressed.append([BYTE_COUNT_DELIM, 255, BYTE_COUNT_DELIM])

        data.advance(num_zeroes if num_zeros < 256 else 255)

    return compressed
```

// describe the use of macros for generating/deriving middleware and other code
== Code generation
Due to the large amount of boilerplate code required to support the implementation of a remote interface, attribute macros are used to generate most of the code at compile-time.

The attribute macros `#[remote_interface]` are placed at the top of the interface definition. This macro is responsible for modifying the original definition, along with the following additional definitions:
- A client-side function to invoke the remote method
- The data type used to represent the request and response of the remote method
- A unique remote method signature, used to dispatch the correct method on the server side.
- A remotely invokable trait for context managers to send and receive requests.

// describe the middleware logic, context manager and dispatcher
== Middleware
The middleware layer handles bidirectional communication between a client and the remote.
This layer serves to abstract away any network-related logic from the implementation of the remote interface.
There are two main parts to the middleware layer: the context manager and the dispatcher.

// describe how the context manager works
=== Context manager
The context manager is a client-side object that comminucates with the dispatcher across the network.

// describe how the dispatcher works
=== Dispatch <dispatch>
The dispatcher is the complement to the context manager. It is responsible for receiving and dispatching requests from context managers across the network.

Once the dispatcher receives a request, it will match the request's method signature with all registered remote interfaces and execute the request.
```py
# pseudocode
def dispatch(self):
    # dispatch loops indefinitely
    while True:
        (data: bytes, addr: SocketAddrV4) = self.proto.recv_bytes()
        middleware_data = deserialize(data)

        # the context manager sends the data wrapped in its own packet
        response = match middleware_data:

            Payload(payload):
                # the handler checks against all registered method signatures
                # and dispatches the request to the correct method
                self.handler.handle_payload(payload)

            other:
                # misc actions
                perform_other_action(other)

        response_packet = serialize(response)
        self.proto.send_bytes(response_packet, addr)

# implementation for handler
def handle_payload(self, payload: bytes) -> bytes:

    remote_interface = self.interfaces.find_method_signature(payload)

    # invoke the method
    response = remote_interface.invoke(payload)

    return response
```

// describe how the UDP limit is circumvented
// when using HandshakeProtocol
=== Transmission protocol <transmission_protocol>
The dispatcher, context manager and remote objects require a transmission protocol to send and receive messages.
```rs
pub struct Dispatcher<H, T>
where
    H: PayloadHandler,
    T: TransmissionProtocol
{/*...*/}

pub struct ContextManager<T: TransmissionProtocol> {/*...*/}

#[async_trait]
pub trait TransmissionProtocol {
    /// Send bytes to the remote.
    async fn send_bytes(
        &mut self,
        sock: &UdpSocket,
        target: SocketAddrV4,
        payload: &[u8],
        timeout: Duration,
        retries: u8,
    ) -> io::Result<usize>;

    /// Wait for a UDP packet. Returns the packet source and data.
    async fn recv_bytes(
        &mut self,
        sock: &UdpSocket,
        timeout: Duration,
        retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)>;
}
```

Each of the protocols described in @transmission_protocol along with the dispatcher in @dispatch fulfill the requirements for various invocation semantics.
The following table describes the invocation semantics fulfilled by each protocol.
The faulty protocols omit the transmission of packets based on a set probability to simulate network errors.

#figure(
    caption: [Invocation semantics fulfilled by each protocol],
    table(
        align: left,
        columns: (auto, auto, auto, auto),
            [*Semantics*], [*protocol*], [*faulty protocol*], [*explanation*],
            [Maybe], [`DefaultProto`], [`DefaultFaultyProto`], [
                Basic UDP messaging does not guarantee the receipt of a packet. Performs simple data compression and decompression.
            ],
            [At-least-once], [`RequestAckProto`], [`RequestAckFaultyProto`], [
                This implementation includes timeouts and retries to ensure the remote receives the packet at least once.
            ],
            [At-most-once], [`HandshakeProto`], [`HandshakeFaultyProto`], [
                To completely fulfill at-most-once semantics, the dispatcher is also configured to filter duplicate requests. The protocol also supports the transmission of arbitrary sized payloads.
            ],
    )
) <protocol_table>

// === Maybe invocation semantics
// `DefaultProto` and `DefaultFaultyProto` fulfill the requirements for maybe invocation semantics.
// As there is no acknowledgement of the receipt of a packet, the remote may or may not receive the packet.
// The same issue arises when the remote sends a response back to the client.


// === At-least-once invocation semantics
// === At-most-once invocation semantics



// report on the results of the experiments:
// - at-most-once invocation semantics
// - at-least-once invocation semantics
//
// what's expected:
// at-least-once can lead to wrong results for non-idempotent operations
// at-most-once work correctly for all operations
= Experiments
The experiments described below aim to determine the success and correctness of the protocols defined in @protocol_table. The primary goals are to:
- Determine the success rate of each protocol (e.g. a value is returned from the remote after a request is sent)
- Determine the correctness of each protocol (e.g. the value returned from the remote is the expected value for idempotent and non-idempotent operations)

// table of experiments to perform
#figure(
    caption: [Various experiments to perform],
    table(
        align: left,
        columns: (auto, auto, auto),
            [*Experiment*], [*Aim*], [*Description*],
            [1], [Control], [Test with no simulated ommision failures],
            [2], [Client-side failure], [Simulate network failures on the client only],
            [3], [Server-side failure], [Simulate network failures on the server only],
            [4], [Twin failure], [Simulate network failures on both the client and server],
    )
) <experiment_desc_table>

== Results
In an ideal network environment, all protocols perform their operations reliably.

From the data shown in @plot_overview, HandshakeProto (at-most-once) is the only protocol that guarantees the correct result for all operations.
However, due to the number of intermediate data transmissions required to ensure at-most-once semantics, the protocol experiences a high failure rate at a log inverse probability, $1 / 10^N$ at $N = 1$ , or $10%$ for every socket transmission.

At lower simulated failure rates, HandshakeProto becomes more reliable than RequestAckProto (at-least-once).
RequestAckProto also experiences non-idempotent violations, as shown in @idem_overview.

#figure(
    caption: [Overview of failure rates for each protocol],
    image(
        "media/plot.svg",
    )
) <plot_overview>

#figure(
    caption: [Non idempotent operation results],
    image(
        "media/idem.svg",
    )
) <idem_overview>
