#![forbid(unsafe_op_in_unsafe_fn)]

use abi_stable::std_types::{RResult, RString, RVec};

mod obj;
mod gltf;
mod fbx;

pub(crate) trait Provider: Sync {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn sniff(&self, bytes: &[u8]) -> bool;

    fn import(&self, bytes: &[u8]) -> RResult<RVec<u8>, RString>;

    /// Returns a JSON object string that describes the format.
    fn describe_json(&self) -> &'static str;
}

pub(crate) fn iter_providers() -> impl Iterator<Item=&'static dyn Provider> {
    static OBJ: obj::ObjProvider = obj::ObjProvider;
    static GLTF: gltf::GltfProvider = gltf::GltfProvider;
    static FBX: fbx::FbxProvider = fbx::FbxProvider;

    [
        &OBJ as &dyn Provider,
        &GLTF as &dyn Provider,
        &FBX as &dyn Provider,
    ]
        .into_iter()
}
