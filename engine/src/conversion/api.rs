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

use crate::{
    additional_cpp_generator::AdditionalNeed,
    types::{Namespace, TypeName},
};
use std::{collections::HashSet, fmt::Display};
use syn::{ForeignItem, Ident, Item};

#[derive(Debug)]
pub enum ConvertError {
    NoContent,
    UnsafePODType(String),
    UnexpectedForeignItem,
    UnexpectedOuterItem,
    UnexpectedItemInMod,
    ComplexTypedefTarget(String),
    UnexpectedThisType,
}

impl Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConvertError::NoContent => write!(f, "The initial run of 'bindgen' did not generate any content. This might be because none of the requested items for generation could be converted.")?,
            ConvertError::UnsafePODType(err) => write!(f, "An item was requested using 'generate_pod' which was not safe to hold by value in Rust. {}", err)?,
            ConvertError::UnexpectedForeignItem => write!(f, "Bindgen generated some unexpected code in a foreign mod section. You may have specified something in a 'generate' directive which is not currently compatible with autocxx.")?,
            ConvertError::UnexpectedOuterItem => write!(f, "Bindgen generated some unexpected code in its outermost mod section. You may have specified something in a 'generate' directive which is not currently compatible with autocxx.")?,
            ConvertError::UnexpectedItemInMod => write!(f, "Bindgen generated some unexpected code in an inner namespace mod. You may have specified something in a 'generate' directive which is not currently compatible with autocxx.")?,
            ConvertError::ComplexTypedefTarget(ty) => write!(f, "autocxx was unable to produce a typdef pointing to the complex type {}.", ty)?,
            ConvertError::UnexpectedThisType => write!(f, "Unexpected type for 'this'")?, // TODO give type/function
        }
        Ok(())
    }
}

/// Whetther and how this type should be exposed in the mods constructed
/// for actual end-user use.
pub(crate) enum Use {
    Unused,
    Used,
    UsedWithAlias(Ident),
}

/// Any API we encounter in the input bindgen rs which we might want to pass
/// onto the output Rust or C++. Everything is stored in these structures
/// because we will do a garbage collection for unnecessary APIs later,
/// using the `deps` field as the edges in the graph.
pub(crate) struct Api {
    pub(crate) ns: Namespace,
    pub(crate) id: Ident,
    pub(crate) use_stmt: Use,
    pub(crate) deps: HashSet<TypeName>,
    pub(crate) extern_c_mod_item: Option<ForeignItem>,
    pub(crate) bridge_item: Option<Item>,
    pub(crate) global_items: Vec<Item>,
    pub(crate) additional_cpp: Option<AdditionalNeed>,
    pub(crate) id_for_allowlist: Option<Ident>,
    pub(crate) bindgen_mod_item: Option<Item>,
}

impl Api {
    pub(crate) fn typename(&self) -> TypeName {
        TypeName::new(&self.ns, &self.id.to_string())
    }

    pub(crate) fn typename_for_allowlist(&self) -> TypeName {
        let id_for_allowlist = match &self.id_for_allowlist {
            None => match &self.use_stmt {
                Use::UsedWithAlias(alias) => alias,
                _ => &self.id,
            },
            Some(id) => &id,
        };
        TypeName::new(&self.ns, &id_for_allowlist.to_string())
    }
}