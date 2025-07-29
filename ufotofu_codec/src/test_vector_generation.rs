/// This module provides fuzz macros for creating corpuses which constitute good test vectors, and functions which read those corpora and create separate directories of test vectors suitable for consumption from other programming languages.
use std::fmt::Debug;
use std::fs::read_dir;
use std::path::Path;
use std::string::ToString;
use std::vec::Vec;
use std::{boxed::Box, fs};
use std::{format, io};

use arbitrary::{Arbitrary, Unstructured};

use crate::{Decodable, DecodableCanonic, Encodable, RelativeDecodable, RelativeDecodableCanonic};

/// A macro for running fuzz tests whose corpora are varied test inputs for absolute decoding of an encoding relation. Usage:
///
/// ```ignore
/// #![no_main]
///
/// use ufotofu_codec::fuzz_absolute_corpus;
///
/// fuzz_absolute_corpus!(path::to_some::TypeToGenerateDataFor);
/// ```
///
/// Assumes that `libfuzzer_sys`, `ufotofu`, and `ufotofu_codec` modules are available.
#[macro_export]
macro_rules! fuzz_absolute_corpus {
    ($t:ty) => {
        use libfuzzer_sys::fuzz_target;
        use ufotofu_codec::Decodable;

        fuzz_target!(|data: Box<[u8]>| {
            pollster::block_on(async {
                let _decoded = <$t as Decodable>::decode_from_slice(&data[..]).await;
            });
        });
    };
}

/// Given the path to a fuzz corpus created with the [`fuzz_absolute_corpus`] macro, this function writes to the `out_dir` a set of test vectors.
pub async fn generate_test_vectors_absolute<T, P1: AsRef<Path>, P2: AsRef<Path>>(
    corpus_dir: P1,
    out_dir: P2,
) where
    T: Encodable + Decodable + Debug,
    T::ErrorReason: Debug,
{
    let mut i = 0;

    for corpus_file in read_dir(corpus_dir).unwrap() {
        let data = fs::read(corpus_file.unwrap().path()).unwrap();
        single_test_vector_absolute::<T, _>(&data[..], out_dir.as_ref(), i).await;
        i += 1;
    }
}

/// Takes the bytes of a corpus file of a `fuzz_absolute_corpus` macro, the directory in which to place test vector data, and a counter to number the generated vector, and writes the test vector data to the file system.
async fn single_test_vector_absolute<T, P: AsRef<Path>>(
    unstructured_bytes: &[u8],
    out_dir: P,
    count: usize,
) where
    T: Encodable + Decodable + Debug,
    T::ErrorReason: Debug,
{
    let mut u = Unstructured::new(unstructured_bytes);
    let bytes: Box<[u8]> = Arbitrary::arbitrary(&mut u).unwrap();

    match T::decode_from_slice(&bytes[..]).await {
        Ok(decoded) => {
            let mut path_input = out_dir.as_ref().join("yay");
            path_input.push(count.to_string());
            write_file_create_parent_dirs(&path_input, bytes).unwrap();

            let mut path_debug_representation = out_dir.as_ref().join("dbg");
            path_debug_representation.push(count.to_string());
            write_file_create_parent_dirs(&path_debug_representation, format!("{:#?}", decoded))
                .unwrap();

            let mut path_reencoded = out_dir.as_ref().join("reencoded");
            path_reencoded.push(count.to_string());
            write_file_create_parent_dirs(&path_reencoded, decoded.encode_into_vec().await)
                .unwrap();
        }
        Err(reason) => {
            let mut path_input = out_dir.as_ref().join("nay");
            path_input.push(count.to_string());
            write_file_create_parent_dirs(&path_input, bytes).unwrap();

            let mut path_nay_reason = out_dir.as_ref().join("nay_reason");
            path_nay_reason.push(count.to_string());
            write_file_create_parent_dirs(&path_nay_reason, format!("{:#?}", reason)).unwrap();
        }
    }
}

/// A macro for running fuzz tests whose corpora are varied test inputs for absolute decoding of an encoding function. Usage:
///
/// ```ignore
/// #![no_main]
///
/// use ufotofu_codec::fuzz_absolute_corpus;
///
/// fuzz_absolute_canonic_corpus!(path::to_some::TypeToGenerateDataFor);
/// ```
///
/// Assumes that `libfuzzer_sys`, `ufotofu`, and `ufotofu_codec` modules are available.
#[macro_export]
macro_rules! fuzz_absolute_canonic_corpus {
    ($t:ty) => {
        use libfuzzer_sys::fuzz_target;
        use ufotofu_codec::DecodableCanonic;

        fuzz_target!(|data: Box<[u8]>| {
            pollster::block_on(async {
                let _decoded = <$t as DecodableCanonic>::decode_canonic_from_slice(&data[..]).await;
            });
        });
    };
}

/// Given the path to a fuzz corpus created with the [`fuzz_absolute_canonic_corpus`] macro, this function writes to the `out_dir` a set of test vectors.
pub async fn generate_test_vectors_canonic_absolute<T, P1: AsRef<Path>, P2: AsRef<Path>>(
    corpus_dir: P1,
    out_dir: P2,
) where
    T: Encodable + DecodableCanonic + Debug,
    T::ErrorCanonic: Debug,
{
    let mut i = 0;

    for corpus_file in read_dir(corpus_dir).unwrap() {
        let data = fs::read(corpus_file.unwrap().path()).unwrap();
        single_test_vector_canonic_absolute::<T, _>(&data[..], out_dir.as_ref(), i).await;
        i += 1;
    }
}

/// Takes the bytes of a corpus file of a `fuzz_absolute_corpus` macro, the directory in which to place test vector data, and a counter to number the generated vector, and writes the test vector data to the file system.
async fn single_test_vector_canonic_absolute<T, P: AsRef<Path>>(
    unstructured_bytes: &[u8],
    out_dir: P,
    count: usize,
) where
    T: Encodable + DecodableCanonic + Debug,
    T::ErrorCanonic: Debug,
{
    let mut u = Unstructured::new(unstructured_bytes);
    let bytes: Box<[u8]> = Arbitrary::arbitrary(&mut u).unwrap();

    match T::decode_canonic_from_slice(&bytes[..]).await {
        Ok(decoded) => {
            let mut path_input = out_dir.as_ref().join("yay");
            path_input.push(count.to_string());
            write_file_create_parent_dirs(&path_input, bytes).unwrap();

            let mut path_debug_representation = out_dir.as_ref().join("dbg");
            path_debug_representation.push(count.to_string());
            write_file_create_parent_dirs(&path_debug_representation, format!("{:#?}", decoded))
                .unwrap();
        }
        Err(reason) => {
            let mut path_input = out_dir.as_ref().join("nay");
            path_input.push(count.to_string());
            write_file_create_parent_dirs(&path_input, bytes).unwrap();

            let mut path_nay_reason = out_dir.as_ref().join("nay_reason");
            path_nay_reason.push(count.to_string());
            write_file_create_parent_dirs(&path_nay_reason, format!("{:#?}", reason)).unwrap();
        }
    }
}

/// A macro for running fuzz tests whose corpora are varied test inputs for decoding of a relative encoding relation. Usage:
///
/// ```ignore
/// #![no_main]
///
/// use ufotofu_codec::fuzz_relative_corpus;
///
/// fuzz_relative_corpus!(path::to_some::TypeToGenerateDataFor, path::to::RelativeToType);
/// ```
///
/// Assumes that `libfuzzer_sys`, `ufotofu`, and `ufotofu_codec` modules are available.
#[macro_export]
macro_rules! fuzz_relative_corpus {
    ($t:ty, $r:ty) => {
        use libfuzzer_sys::fuzz_target;
        use ufotofu_codec::RelativeDecodable;

        fuzz_target!(|data: (Box<[u8]>, $r)| {
            let (bytes, r) = data;

            pollster::block_on(async {
                let _decoded =
                    <$t as RelativeDecodable<$r, _>>::relative_decode_from_slice(&bytes[..], &r)
                        .await;
            });
        });
    };
}

/// Given the path to a fuzz corpus created with the [`fuzz_relative_corpus`] macro, this function writes to the `out_dir` a set of test vectors.
pub async fn generate_test_vectors_relative<T, R, ErrorReason, P1: AsRef<Path>, P2: AsRef<Path>>(
    corpus_dir: P1,
    out_dir: P2,
) where
    T: Encodable + RelativeDecodable<R, ErrorReason> + Debug,
    R: Encodable + Debug + for<'a> Arbitrary<'a>,
    ErrorReason: Debug,
{
    let mut i = 0;

    for corpus_file in read_dir(corpus_dir).unwrap() {
        let data = fs::read(corpus_file.unwrap().path()).unwrap();
        single_test_vector_relative::<T, R, ErrorReason, _>(&data[..], out_dir.as_ref(), i).await;
        i += 1;
    }
}

/// Takes the bytes of a corpus file of a `fuzz_relative_corpus` macro, the directory in which to place test vector data, and a counter to number the generated vector, and writes the test vector data to the file system.
async fn single_test_vector_relative<T, R, ErrorReason, P: AsRef<Path>>(
    unstructured_bytes: &[u8],
    out_dir: P,
    count: usize,
) where
    T: Encodable + RelativeDecodable<R, ErrorReason> + Debug,
    R: Encodable + Debug + for<'a> Arbitrary<'a>,
    ErrorReason: Debug,
{
    let mut u = Unstructured::new(unstructured_bytes);
    let (bytes, r): (Box<[u8]>, R) = Arbitrary::arbitrary(&mut u).unwrap();

    match T::relative_decode_from_slice(&bytes[..], &r).await {
        Ok(decoded) => {
            let mut path_input = out_dir.as_ref().join("yay");
            path_input.push(count.to_string());
            write_file_create_parent_dirs(&path_input, bytes).unwrap();

            let mut path_input_rel = out_dir.as_ref().join("yay_relative_to");
            path_input_rel.push(count.to_string());
            write_file_create_parent_dirs(&path_input_rel, r.encode_into_vec().await).unwrap();

            let pair = EncodedPair {
                actual_value: decoded,
                relative_to: r,
            };

            let mut path_debug_representation = out_dir.as_ref().join("dbg");
            path_debug_representation.push(count.to_string());
            write_file_create_parent_dirs(&path_debug_representation, format!("{:#?}", pair))
                .unwrap();

            let mut path_reencoded = out_dir.as_ref().join("reencoded");
            path_reencoded.push(count.to_string());
            write_file_create_parent_dirs(
                &path_reencoded,
                pair.actual_value.encode_into_vec().await,
            )
            .unwrap();
        }
        Err(reason) => {
            let mut path_input = out_dir.as_ref().join("nay");
            path_input.push(count.to_string());
            write_file_create_parent_dirs(&path_input, bytes).unwrap();

            let mut path_input_rel = out_dir.as_ref().join("nay_relative_to");
            path_input_rel.push(count.to_string());
            write_file_create_parent_dirs(&path_input_rel, r.encode_into_vec().await).unwrap();

            let mut path_debug_representation = out_dir.as_ref().join("nay_dbg");
            path_debug_representation.push(count.to_string());
            write_file_create_parent_dirs(&path_debug_representation, format!("{:#?}", r)).unwrap();

            let mut path_nay_reason = out_dir.as_ref().join("nay_reason");
            path_nay_reason.push(count.to_string());
            write_file_create_parent_dirs(&path_nay_reason, format!("{:#?}", reason)).unwrap();
        }
    }
}

/// Given the path to a fuzz corpus created with the [`fuzz_relative_corpus`] macro, this function writes to the `out_dir` a set of test vectors. Takes a function for encoding relative values instead of requiring `Encodable`.
pub async fn generate_test_vectors_relative_custom_serialisation<
    T,
    R,
    ErrorReason,
    P1: AsRef<Path>,
    P2: AsRef<Path>,
>(
    corpus_dir: P1,
    out_dir: P2,
    encode_r: fn(&R) -> Vec<u8>,
) where
    T: Encodable + RelativeDecodable<R, ErrorReason> + Debug,
    R: Debug + for<'a> Arbitrary<'a>,
    ErrorReason: Debug,
{
    let mut i = 0;

    for corpus_file in read_dir(corpus_dir).unwrap() {
        let data = fs::read(corpus_file.unwrap().path()).unwrap();
        single_test_vector_relative_custom_serialisation::<T, R, ErrorReason, _>(
            &data[..],
            out_dir.as_ref(),
            i,
            encode_r,
        )
        .await;
        i += 1;
    }
}

/// Takes the bytes of a corpus file of a `fuzz_relative_corpus` macro, the directory in which to place test vector data, and a counter to number the generated vector, and writes the test vector data to the file system. Takes a function for encoding relative values instead of requiring `Encodable`.
async fn single_test_vector_relative_custom_serialisation<T, R, ErrorReason, P: AsRef<Path>>(
    unstructured_bytes: &[u8],
    out_dir: P,
    count: usize,
    encode_r: fn(&R) -> Vec<u8>,
) where
    T: Encodable + RelativeDecodable<R, ErrorReason> + Debug,
    R: Debug + for<'a> Arbitrary<'a>,
    ErrorReason: Debug,
{
    let mut u = Unstructured::new(unstructured_bytes);
    let (bytes, r): (Box<[u8]>, R) = Arbitrary::arbitrary(&mut u).unwrap();

    match T::relative_decode_from_slice(&bytes[..], &r).await {
        Ok(decoded) => {
            let mut path_input = out_dir.as_ref().join("yay");
            path_input.push(count.to_string());
            write_file_create_parent_dirs(&path_input, bytes).unwrap();

            let mut path_input_rel = out_dir.as_ref().join("yay_relative_to");
            path_input_rel.push(count.to_string());
            write_file_create_parent_dirs(&path_input_rel, encode_r(&r)).unwrap();

            let pair = EncodedPair {
                actual_value: decoded,
                relative_to: r,
            };

            let mut path_debug_representation = out_dir.as_ref().join("dbg");
            path_debug_representation.push(count.to_string());
            write_file_create_parent_dirs(&path_debug_representation, format!("{:#?}", pair))
                .unwrap();

            let mut path_reencoded = out_dir.as_ref().join("reencoded");
            path_reencoded.push(count.to_string());
            write_file_create_parent_dirs(
                &path_reencoded,
                pair.actual_value.encode_into_vec().await,
            )
            .unwrap();
        }
        Err(reason) => {
            let mut path_input = out_dir.as_ref().join("nay");
            path_input.push(count.to_string());
            write_file_create_parent_dirs(&path_input, bytes).unwrap();

            let mut path_input_rel = out_dir.as_ref().join("nay_relative_to");
            path_input_rel.push(count.to_string());
            write_file_create_parent_dirs(&path_input_rel, encode_r(&r)).unwrap();

            let mut path_debug_representation = out_dir.as_ref().join("nay_dbg");
            path_debug_representation.push(count.to_string());
            write_file_create_parent_dirs(&path_debug_representation, format!("{:#?}", r)).unwrap();

            let mut path_nay_reason = out_dir.as_ref().join("nay_reason");
            path_nay_reason.push(count.to_string());
            write_file_create_parent_dirs(&path_nay_reason, format!("{:#?}", reason)).unwrap();
        }
    }
}

/// A macro for running fuzz tests whose corpora are varied test inputs for decoding of a relative encoding function. Usage:
///
/// ```ignore
/// #![no_main]
///
/// use ufotofu_codec::fuzz_relative_canonic_corpus;
///
/// fuzz_relative_canonic_corpus!(path::to_some::TypeToGenerateDataFor, path::to::RelativeToType);
/// ```
///
/// Assumes that `libfuzzer_sys`, `ufotofu`, and `ufotofu_codec` modules are available.
#[macro_export]
macro_rules! fuzz_relative_canonic_corpus {
    ($t:ty, $r:ty) => {
        use libfuzzer_sys::fuzz_target;
        use ufotofu_codec::RelativeDecodableCanonic;

        fuzz_target!(|data: (Box<[u8]>, $r)| {
            let (bytes, r) = data;

            pollster::block_on(async {
                let _decoded =
                    <$t as RelativeDecodableCanonic<$r, _, _>>::relative_decode_canonic_from_slice(
                        &bytes[..],
                        &r,
                    )
                    .await;
            });
        });
    };
}

/// Given the path to a fuzz corpus created with the [`fuzz_relative_canonic_corpus`] macro, this function writes to the `out_dir` a set of test vectors.
pub async fn generate_test_vectors_canonic_relative<
    T,
    R,
    ErrorReason,
    ErrorCanonic,
    P1: AsRef<Path>,
    P2: AsRef<Path>,
>(
    corpus_dir: P1,
    out_dir: P2,
) where
    T: Encodable + RelativeDecodableCanonic<R, ErrorReason, ErrorCanonic> + Debug,
    R: Encodable + Debug + for<'a> Arbitrary<'a>,
    ErrorCanonic: Debug + From<ErrorReason>,
{
    let mut i = 0;

    for corpus_file in read_dir(corpus_dir).unwrap() {
        let data = fs::read(corpus_file.unwrap().path()).unwrap();
        single_test_vector_canonic_relative::<T, R, ErrorReason, ErrorCanonic, _>(
            &data[..],
            out_dir.as_ref(),
            i,
        )
        .await;
        i += 1;
    }
}

/// Takes the bytes of a corpus file of a `fuzz_relative_corpus` macro, the directory in which to place test vector data, and a counter to number the generated vector, and writes the test vector data to the file system.
async fn single_test_vector_canonic_relative<T, R, ErrorReason, ErrorCanonic, P: AsRef<Path>>(
    unstructured_bytes: &[u8],
    out_dir: P,
    count: usize,
) where
    T: Encodable + RelativeDecodableCanonic<R, ErrorReason, ErrorCanonic> + Debug,
    R: Encodable + Debug + for<'a> Arbitrary<'a>,
    ErrorCanonic: Debug + From<ErrorReason>,
{
    let mut u = Unstructured::new(unstructured_bytes);
    let (bytes, r): (Box<[u8]>, R) = Arbitrary::arbitrary(&mut u).unwrap();

    match T::relative_decode_canonic_from_slice(&bytes[..], &r).await {
        Ok(decoded) => {
            let mut path_input = out_dir.as_ref().join("yay");
            path_input.push(count.to_string());
            write_file_create_parent_dirs(&path_input, bytes).unwrap();

            let mut path_input_rel = out_dir.as_ref().join("yay_relative_to");
            path_input_rel.push(count.to_string());
            write_file_create_parent_dirs(&path_input_rel, r.encode_into_vec().await).unwrap();

            let pair = EncodedPair {
                actual_value: decoded,
                relative_to: r,
            };

            let mut path_debug_representation = out_dir.as_ref().join("dbg");
            path_debug_representation.push(count.to_string());
            write_file_create_parent_dirs(&path_debug_representation, format!("{:#?}", pair))
                .unwrap();

            let mut path_reencoded = out_dir.as_ref().join("reencoded");
            path_reencoded.push(count.to_string());
            write_file_create_parent_dirs(
                &path_reencoded,
                pair.actual_value.encode_into_vec().await,
            )
            .unwrap();
        }
        Err(reason) => {
            let mut path_input = out_dir.as_ref().join("nay");
            path_input.push(count.to_string());
            write_file_create_parent_dirs(&path_input, bytes).unwrap();

            let mut path_input_rel = out_dir.as_ref().join("nay_relative_to");
            path_input_rel.push(count.to_string());
            write_file_create_parent_dirs(&path_input_rel, r.encode_into_vec().await).unwrap();

            let mut path_debug_representation = out_dir.as_ref().join("nay_dbg");
            path_debug_representation.push(count.to_string());
            write_file_create_parent_dirs(&path_debug_representation, format!("{:#?}", r)).unwrap();

            let mut path_nay_reason = out_dir.as_ref().join("nay_reason");
            path_nay_reason.push(count.to_string());
            write_file_create_parent_dirs(&path_nay_reason, format!("{:#?}", reason)).unwrap();
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct EncodedPair<T, R> {
    actual_value: T,
    relative_to: R, // This is only there for the Debug impl.
}

fn write_file_create_parent_dirs<P, C>(file: P, contents: C) -> io::Result<()>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    let prefix = file.as_ref().parent().unwrap();
    std::fs::create_dir_all(prefix).unwrap();

    fs::write(file, contents)
}
