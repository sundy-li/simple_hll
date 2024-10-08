use crate::HyperLogLog;

#[derive(serde::Serialize, borsh::BorshSerialize)]
enum HyperLogLogVariantRef<'a> {
    Empty,
    Sparse { data: Vec<(u16, u8)> },
    Full(&'a Vec<u8>),
}

#[derive(serde::Deserialize, borsh::BorshDeserialize)]
enum HyperLogLogVariant {
    Empty,
    Sparse { data: Vec<(u16, u8)> },
    Full(Vec<u8>),
}

impl<const P: usize> From<HyperLogLogVariant> for HyperLogLog<P> {
    fn from(value: HyperLogLogVariant) -> Self {
        match value {
            HyperLogLogVariant::Empty => HyperLogLog::<P>::new(),
            HyperLogLogVariant::Sparse { data } => {
                let mut registers = vec![0; 1 << P];
                for (index, val) in data {
                    registers[index as usize] = val;
                }

                HyperLogLog::<P>::with_registers(registers)
            }
            HyperLogLogVariant::Full(registers) => HyperLogLog::<P>::with_registers(registers),
        }
    }
}

impl<'a, const P: usize> From<&'a HyperLogLog<P>> for HyperLogLogVariantRef<'a> {
    fn from(hll: &'a HyperLogLog<P>) -> Self {
        let none_empty_registers = HyperLogLog::<P>::number_registers() - hll.num_empty_registers();

        if none_empty_registers == 0 {
            HyperLogLogVariantRef::Empty
        } else if none_empty_registers * 3 <= HyperLogLog::<P>::number_registers() {
            // If the number of empty registers is larger enough, we can use sparse serialize to reduce the binary size
            // each register in sparse format will occupy 3 bytes, 2 for register index and 1 for register value.
            let sparse_data: Vec<(u16, u8)> = hll
                .registers
                .iter()
                .enumerate()
                .filter_map(|(index, &value)| {
                    if value != 0 {
                        Some((index as u16, value))
                    } else {
                        None
                    }
                })
                .collect();

            HyperLogLogVariantRef::Sparse { data: sparse_data }
        } else {
            HyperLogLogVariantRef::Full(&hll.registers)
        }
    }
}

impl<const P: usize> serde::Serialize for HyperLogLog<P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let v: HyperLogLogVariantRef<'_> = self.into();
        v.serialize(serializer)
    }
}

impl<'de, const P: usize> serde::Deserialize<'de> for HyperLogLog<P> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = HyperLogLogVariant::deserialize(deserializer)?;
        Ok(v.into())
    }
}

impl<const P: usize> borsh::BorshSerialize for HyperLogLog<P> {
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let v: HyperLogLogVariantRef<'_> = self.into();
        v.serialize(writer)
    }
}

impl<const P: usize> borsh::BorshDeserialize for HyperLogLog<P> {
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
        let v = HyperLogLogVariant::deserialize_reader(reader)?;
        Ok(v.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::HyperLogLog;

    const P: usize = 14;

    #[test]
    fn test_serde() {
        let mut hll = HyperLogLog::<P>::new();
        json_serde_equal(&hll);

        for i in 0..100000 {
            hll.add_object(&(i % 200));
        }
        json_serde_equal(&hll);

        let hll = HyperLogLog::<P>::with_registers(vec![1; 1 << P]);
        json_serde_equal(&hll);
    }

    fn json_serde_equal<T>(t: &T)
    where
        T: serde::Serialize + for<'a> serde::Deserialize<'a> + Eq,
    {
        let val = serde_json::to_vec(t).unwrap();
        let new_t: T = serde_json::from_slice(&val).unwrap();
        assert!(t == &new_t)
    }

    // #[test]
    // #[cfg(target_os = "linux")]
    // fn test_back_compat() {
    //     let mut hll = HyperLogLog::<P>::new();

    //     for i in 0..1000000 {
    //         hll.add_object(&i);
    //     }

    //     for i in 0..1000000 {
    //         hll.add_object(&i);
    //     }

    //     let s: u64 = hll.registers.iter().map(|x| *x as u64).sum();
    //     assert_eq!(s, 118702);

    //     for i in 0..2000000 {
    //         hll.add_object(&i);
    //     }
    //     let s: u64 = hll.registers.iter().map(|x| *x as u64).sum();
    //     assert_eq!(s, 135525);
    // }
}
