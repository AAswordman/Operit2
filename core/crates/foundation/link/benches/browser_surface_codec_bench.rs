use std::fmt;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const FRAME_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct JsonBrowserSurfaceFrame {
    streamId: String,
    sequence: u64,
    timestampMicros: u64,
    codec: String,
    keyframe: bool,
    dataBase64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct BinaryBrowserSurfaceFrame {
    streamId: String,
    sequence: u64,
    timestampMicros: u64,
    codec: String,
    keyframe: bool,
    #[serde(with = "binary_bytes")]
    data: Vec<u8>,
}

mod binary_bytes {
    use super::*;

    /// Serializes bytes through the codec's native binary representation.
    pub fn serialize<S>(value: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(value)
    }

    /// Deserializes bytes from the codec's native binary representation.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_byte_buf(BytesVisitor)
    }

    struct BytesVisitor;

    impl<'de> Visitor<'de> for BytesVisitor {
        type Value = Vec<u8>;

        /// Describes the native binary value accepted by this visitor.
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte string")
        }

        /// Copies borrowed bytes into the decoded frame payload.
        fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E> {
            Ok(value.to_vec())
        }

        /// Accepts an owned byte buffer without another conversion.
        fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E> {
            Ok(value)
        }

        /// Accepts sequence-oriented decoders for completeness.
        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut value = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
            while let Some(byte) = sequence.next_element::<u8>()? {
                value.push(byte);
            }
            Ok(value)
        }
    }
}

/// Builds deterministic incompressible-looking bytes representing an encoded video chunk.
fn frame_bytes() -> Vec<u8> {
    let mut state = 0x9e37_79b9_u32;
    (0..FRAME_BYTES)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as u8
        })
        .collect()
}

/// Builds the native-binary browser surface frame used by CBOR and MessagePack.
fn binary_frame() -> BinaryBrowserSurfaceFrame {
    BinaryBrowserSurfaceFrame {
        streamId: "browser-surface-benchmark".to_string(),
        sequence: 42,
        timestampMicros: 1_783_800_000_000_000,
        codec: "h264".to_string(),
        keyframe: false,
        data: frame_bytes(),
    }
}

/// Builds the current JSON and base64 representation of the same surface frame.
fn json_frame(frame: &BinaryBrowserSurfaceFrame) -> JsonBrowserSurfaceFrame {
    JsonBrowserSurfaceFrame {
        streamId: frame.streamId.clone(),
        sequence: frame.sequence,
        timestampMicros: frame.timestampMicros,
        codec: frame.codec.clone(),
        keyframe: frame.keyframe,
        dataBase64: STANDARD.encode(&frame.data),
    }
}

/// Encodes one surface frame through the current JSON and base64 path.
fn encode_json_base64(frame: &BinaryBrowserSurfaceFrame) -> Vec<u8> {
    serde_json::to_vec(&json_frame(frame)).expect("JSON browser frame must encode")
}

/// Decodes one surface frame through the current JSON and base64 path.
fn decode_json_base64(bytes: &[u8]) -> Vec<u8> {
    let frame = serde_json::from_slice::<JsonBrowserSurfaceFrame>(bytes)
        .expect("JSON browser frame must decode");
    STANDARD
        .decode(frame.dataBase64)
        .expect("base64 browser frame must decode")
}

/// Encodes one surface frame as CBOR with a native byte string.
fn encode_cbor(frame: &BinaryBrowserSurfaceFrame) -> Vec<u8> {
    let mut output = Vec::new();
    ciborium::ser::into_writer(frame, &mut output).expect("CBOR browser frame must encode");
    output
}

/// Decodes one native-byte CBOR surface frame.
fn decode_cbor(bytes: &[u8]) -> BinaryBrowserSurfaceFrame {
    ciborium::de::from_reader(bytes).expect("CBOR browser frame must decode")
}

/// Encodes one surface frame as MessagePack with a native bin value.
fn encode_message_pack(frame: &BinaryBrowserSurfaceFrame) -> Vec<u8> {
    operit_link::encodeLink(frame).expect("MessagePack browser frame must encode")
}

/// Decodes one native-bin MessagePack surface frame.
fn decode_message_pack(bytes: &[u8]) -> BinaryBrowserSurfaceFrame {
    operit_link::decodeLink(bytes).expect("MessagePack browser frame must decode")
}

/// Registers browser surface codec throughput and wire-size comparisons.
fn bench_browser_surface_codecs(criterion: &mut Criterion) {
    let frame = binary_frame();
    let json_bytes = encode_json_base64(&frame);
    let cbor_bytes = encode_cbor(&frame);
    let message_pack_bytes = encode_message_pack(&frame);

    println!(
        "browser surface wire bytes: raw={} json_base64={} cbor={} msgpack={}",
        frame.data.len(),
        json_bytes.len(),
        cbor_bytes.len(),
        message_pack_bytes.len()
    );

    let mut encode_group = criterion.benchmark_group("browser surface encode 256KiB");
    encode_group.throughput(Throughput::Bytes(FRAME_BYTES as u64));
    encode_group.bench_with_input(
        BenchmarkId::new("json_base64", FRAME_BYTES),
        &frame,
        |b, frame| {
            b.iter(|| encode_json_base64(black_box(frame)));
        },
    );
    encode_group.bench_with_input(
        BenchmarkId::new("cbor_bytes", FRAME_BYTES),
        &frame,
        |b, frame| {
            b.iter(|| encode_cbor(black_box(frame)));
        },
    );
    encode_group.bench_with_input(
        BenchmarkId::new("msgpack_bytes", FRAME_BYTES),
        &frame,
        |b, frame| {
            b.iter(|| encode_message_pack(black_box(frame)));
        },
    );
    encode_group.finish();

    let mut decode_group = criterion.benchmark_group("browser surface decode 256KiB");
    decode_group.throughput(Throughput::Bytes(FRAME_BYTES as u64));
    decode_group.bench_with_input(
        BenchmarkId::new("json_base64", json_bytes.len()),
        &json_bytes,
        |b, bytes| b.iter(|| decode_json_base64(black_box(bytes))),
    );
    decode_group.bench_with_input(
        BenchmarkId::new("cbor_bytes", cbor_bytes.len()),
        &cbor_bytes,
        |b, bytes| b.iter(|| decode_cbor(black_box(bytes))),
    );
    decode_group.bench_with_input(
        BenchmarkId::new("msgpack_bytes", message_pack_bytes.len()),
        &message_pack_bytes,
        |b, bytes| b.iter(|| decode_message_pack(black_box(bytes))),
    );
    decode_group.finish();
}

criterion_group!(benches, bench_browser_surface_codecs);
criterion_main!(benches);
