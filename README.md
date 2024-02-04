# simple_hll &emsp; [![Build Status]][actions] [![Latest Version]][crates.io]

[Build Status]: https://img.shields.io/github/actions/workflow/status/sundy-li/simple_hll/ci.yml
[actions]: https://github.com/sundy-li/simple_hll/actions?query=branch%3Amain
[Latest Version]: https://img.shields.io/crates/v/simple_hll.svg
[crates.io]: https://crates.io/crates/simple_hll

`simple_hll` is a simple HyperLogLog implementation in rust. It is designed to be simple to use and less bytes to store (with Sparse HyperLogLog).

## Quick Start

```rust
use simple_hll::HyperLogLog;

let mut hll = HyperLogLog::<14>::new();
hll.add_object("hello");
hll.add_object("world");
hll.add_object("simple_hll");

println!("cardinality: {}", hll.count());
```


## Serde
`simple_hll` supports serde and borsh with feature `serde_borsh` enabled, so you can serialize and deserialize the HyperLogLog instance.

```rust
   let val = serde_json::to_vec(hll)?;
```

Notice that in order to reduce the serialized size, we introduce a sparse intermediate struct for the HyperLogLog instance. When the non-zero registers are less than a threshold, we will use the sparse mode to serialize the HyperLogLog instance.

``` rust
enum HyperLogLogVariant<const P: usize> {
    Empty,
    Sparse { data: Vec<(u16, u8)> },
    Full(Vec<u8>),
}
```

## None-Fixed type

Different from other hyperloglog implementation, we don't use fixed type `HyperLogLog<T>` for the HyperLogLog instance, but we use a const generic parameter to specify the precision. The precision `P` is the number of bits to use for the register index. The number of registers is `2^P`. The precision `P` is a trade-off between the accuracy and the memory usage. The default precision is 14, which means the memory usage is about 16KB.

The reason is that in databend or other dbms, we will store the `HyperLogLog` inside the metadata. We don't want to use `HyperLogLog<Datum>` for simplicity and less overhead to hash the enum.

## Contributing

Check out the [CONTRIBUTING.md](./CONTRIBUTING.md) guide for more details on getting started with contributing to this project.

## Acknowledgements

Some codes and tests are borrowed and inspired from:
- [redis](https://github.com/redis/redis/blob/4930d19e70c391750479951022e207e19111eb55/src/hyperloglog.c)
- [datafusion](https://github.com/apache/arrow-datafusion/blob/f203d863f5c8bc9f133f6dd9b2e34e57ac3cdddc/datafusion/physical-expr/src/aggregate/hyperloglog.rs)
- [pdatastructs](https://github.com/crepererum/pdatastructs.rs/blob/3997ed50f6b6871c9e53c4c5e0f48f431405fc63/src/hyperloglog.rs)

Reference papers:
- [New cardinality estimation algorithms for HyperLogLog sketches](https://arxiv.org/abs/1702.01284)


Thanks for the great work of the authors and contributors.

#### License

<sup>
Licensed under <a href="./LICENSE">Apache License, Version 2.0</a>.
</sup>