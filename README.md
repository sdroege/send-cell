# send-cell

An immutable memory location that implements `Send` for types that do not
implement it.

Enforcing safety with regard to the `Send` trait happens at runtime instead of
compile time. Accessing the contained value will call `panic!` if happening
from any thread but the thread on which the value was created on. The
`SendCell` can be safely transferred to other threads.

## LICENSE

gstreamer-sys and all crates contained here are licensed under the MIT
license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT).

## Contribution

Any kinds of contributions are welcome as a pull request.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in send-cell by you shall be licensed under the MIT license as above,
without any additional terms or conditions.
