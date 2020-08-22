// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use syn::Item;
use tempfile::{tempdir, TempDir};

/// Errors returned during creation of a cc::Build from an include_cxx
/// macro.
#[derive(Debug)]
pub enum Error {
    /// The .rs file didn't exist or couldn't be read.
    FileReadError(std::io::Error),
    /// The .rs file couldn't be parsed.
    Syntax(syn::Error),
    /// The cxx module couldn't parse the code generated by autocxx.
    /// This could well be a bug in autocxx.
    InvalidCxx(String),
    /// We couldn't create a temporary directory to store the c++ code.
    TempDirCreationFailed(std::io::Error),
    /// We couldn't write the c++ code to disk.
    FileWriteFail(std::io::Error),
    /// No `include_cxx` macro was found anywhere.
    NoIncludeCxxMacrosFound,
    /// Unable to parse the include_cxx macro.
    MacroParseFail(syn::Error),
}

/// Structure for use in a build.rs file to aid with conversion
/// of a `include_cxx!` macro into a `cc::Build`.
/// This structure owns a temporary directory containing
/// the generated C++ code, as well as owning the cc::Build
/// which knows how to build it.
/// Typically you'd use this from a build.rs file by
/// using `new` and then using `builder` to fetch the `cc::Build`
/// object and asking the resultant `cc::Build` to compile the code.
pub struct Builder {
    build: cc::Build,
    _tdir: TempDir,
}

impl Builder {
    /// Construct a Builder.
    pub fn new(rs_file: impl AsRef<Path>) -> Result<Self, Error> {
        // TODO - we have taken a different approach here from cxx.
        // cxx jumps through many (probably very justifiable) hoops
        // to generate .h and .cxx files in the Cargo out directory
        // (I think). We cheat and just make a temp dir. We shouldn't.
        let tdir = tempdir().map_err(Error::TempDirCreationFailed)?;
        let mut builder = cc::Build::new();
        builder.cpp(true);
        let source = fs::read_to_string(rs_file).map_err(Error::FileReadError)?;
        // TODO - put this macro-finding code into the 'engine'
        // directory such that it can be shared with gen/cmd.
        // However, the use of cc::Build is unique to gen/build.
        let source = syn::parse_file(&source).map_err(Error::Syntax)?;
        let mut counter = 0;
        for item in source.items {
            if let Item::Macro(mac) = item {
                if mac.mac.path.is_ident("include_cxx") {
                    let include_cpp = autocxx_engine::IncludeCpp::new_from_syn(mac.mac)
                        .map_err(Error::MacroParseFail)?;
                    builder.include(include_cpp.include_dir());
                    let (_, cxx) = include_cpp
                        .generate_h_and_cxx()
                        .map_err(Error::InvalidCxx)?;
                    let fname = format!("gen{}.cxx", counter);
                    counter += 1;
                    let gen_cxx_path = Self::write_to_file(&tdir, &fname, &cxx)
                        .map_err(Error::FileWriteFail)?;
                    builder.file(gen_cxx_path);
                }
            }
        }
        if counter == 0 {
            Err(Error::NoIncludeCxxMacrosFound)
        } else {
            Ok(Builder {
                build: builder,
                _tdir: tdir,
            })
        }
    }

    /// Fetch the cc::Build from this.
    pub fn builder(&mut self) -> &mut cc::Build {
        &mut self.build
    }

    fn write_to_file(tdir: &TempDir, filename: &str, content: &[u8]) -> std::io::Result<PathBuf> {
        let path = tdir.path().join(filename);
        let mut f = File::create(&path)?;
        f.write_all(content)?;
        Ok(path)
    }
}
