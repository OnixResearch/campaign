//! JSON value shapes the derived serde impls do not close on their own.
//!
//! A derived struct impl also decodes a positional JSON array and a derived unit-enum impl also decodes a
//! `{"tag": null}` object, but the Nickel export writes every record as an object and every enum tag as a
//! string. These decoders admit records only from JSON objects and tags only from JSON strings, so every
//! other shape is a `Data` decode error instead of an alternate spelling of a valid profile.

/// Decodes a record from a JSON object only; any other JSON value, including an array, is a type error.
pub(crate) fn record<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    deserializer.deserialize_map(RecordVisitor(core::marker::PhantomData))
}

/// Decodes a unit enum tag from a JSON string only; any other JSON value, including an object, is a type error.
pub(crate) fn tag<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    deserializer.deserialize_str(TagVisitor(core::marker::PhantomData))
}

/// Decodes a JSON array of at most `max_tags` unit enum tags, each written as a JSON string.
pub(crate) fn tags<'de, D, T>(deserializer: D, max_tags: usize) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    deserializer.deserialize_seq(TagsVisitor {
        max_tags,
        tag: core::marker::PhantomData,
    })
}

struct RecordVisitor<T>(core::marker::PhantomData<T>);

impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for RecordVisitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("a JSON object record")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
        T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
    }
}

struct TagVisitor<T>(core::marker::PhantomData<T>);

impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for TagVisitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("an enum tag string")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<T, E> {
        T::deserialize(serde::de::value::StrDeserializer::<E>::new(value))
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::de::DeserializeSeed<'de> for TagVisitor<T> {
    type Value = T;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<T, D::Error> {
        deserializer.deserialize_str(self)
    }
}

struct TagsVisitor<T> {
    max_tags: usize,
    tag: core::marker::PhantomData<T>,
}

impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for TagsVisitor<T> {
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "an array of at most {} enum tag strings",
            self.max_tags
        )
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
        let mut values = Vec::with_capacity(self.max_tags);
        while let Some(value) = seq.next_element_seed(TagVisitor(core::marker::PhantomData))? {
            if values.len() >= self.max_tags {
                return Err(serde::de::Error::invalid_length(
                    values.len().saturating_add(1),
                    &self,
                ));
            }
            values.push(value);
        }
        debug_assert!(values.len() <= self.max_tags);
        Ok(values)
    }
}
