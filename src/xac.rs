#![allow(dead_code)]
use crate::tosreader::BinaryReader;
use binrw::{BinRead, binread};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

enum SkeletalMotionType {
    SkelmotiontypeNormal = 0, // A regular keyframe and keytrack based skeletal motion.
    SkelmotiontypeWavelet = 1, // A wavelet compressed skeletal motion.
}

enum FileType {
    FiletypeUnknown = 0,           // An unknown file, or something went wrong.
    FiletypeActor,                 // An actor file (.xac).
    FiletypeSkeletalmotion,        // A skeletal motion file (.xsm).
    FiletypeWaveletskeletalmotion, // A wavelet compressed skeletal motion (.xsm).
    FiletypePmorphmotion,          // A progressive morph motion file (.xpm).
}

// shared chunk ID's
enum SharedChunk {
    SharedChunkMotioneventtable = 50,
    SharedChunkTimestamp = 51,
}

// matrix multiplication order
enum MatrixMulOrder {
    MulorderScaleRotTrans = 0,
    MulorderRotScaleTrans = 1,
}

enum MeshType {
    MeshtypeStatic = 0, //< Static mesh, like a cube or building (can still be position/scale/rotation animated though).
    MeshtypeDynamic = 1, //< Has mesh deformers that have to be processed on the CPU.
    MeshtypeGpuskinned = 2, //< Just a skinning mesh deformer that gets processed on the GPU with skinned shader.
}

enum PhonemeSet {
    PhonemesetNone = 0,
    PhonemesetNeutralPose = 1 << 0,
    PhonemesetMBPX = 1 << 1,
    PhonemesetAaAoOw = 1 << 2,
    PhonemesetIhAeAhEyAyH = 1 << 3,
    PhonemesetAw = 1 << 4,
    PhonemesetNNgChJDhDGTKZZhThSSh = 1 << 5,
    PhonemesetIyEhY = 1 << 6,
    PhonemesetUwUhOy = 1 << 7,
    PhonemesetFV = 1 << 8,
    PhonemesetLEl = 1 << 9,
    PhonemesetW = 1 << 10,
    PhonemesetREr = 1 << 11,
}

enum WaveletType {
    WaveletHaar = 0, // The Haar wavelet, which is most likely what you want to use. It is the fastest also.
    WaveletDaub4 = 1, // Daubechies 4 wavelet, can result in bit better compression ratios, but slower than Haar.
    WaveletCdf97 = 2, // The CDF97 wavelet, used in JPG as well. This is the slowest, but often results in the best compression ratios.
}

enum NodeFlags {
    FlagIncludeinboundscalc = 1 << 0, // Specifies whether we have to include this node in the bounds calculation or not (true on default).
    FlagAttachment = 1 << 1, // Indicates if this node is an attachment node or not (false on default).
}

enum Plane {
    PlaneXy = 0, // The XY plane, so where Z is constant.
    PlaneXz = 1, // The XZ plane, so where Y is constant.
    PlaneYz = 2, // The YZ plane, so where X is constant.
}

enum DependencyType {
    DependencyMeshes = 1 << 0,     // Shared meshes.
    DependencyTransforms = 1 << 1, // Shared transforms.
}

/// The motion based actor repositioning mask
enum RepositioningMask {
    RepositionPosition = 1 << 0, // Update the actor position based on the repositioning node.
    RepositionRotation = 1 << 1, // Update the actor rotation based on the repositioning node.
    RepositionScale = 1 << 2, // [CURRENTLY UNSUPPORTED] Update the actor scale based on the repositioning node.
}

/// The order of multiplication when composing a transformation matrix from a translation, rotation and scale.
enum MultiplicationOrder {
    ScaleRotationTranslation = 0, // LocalTM = scale * rotation * translation (Maya style).
    RotationScaleTranslation = 1, // LocalTM = rotation * scale * translation (3DSMax style) [default].
}

enum LimitType {
    TranslationX = 1 << 0, // Position limit on the x axis.
    TranslationY = 1 << 1, // Position limit on the y axis.
    TranslationZ = 1 << 2, // Position limit on the z axis.
    RotationX = 1 << 3,    // Rotation limit on the x axis.
    RotationY = 1 << 4,    // Rotation limit on the y axis.
    RotationZ = 1 << 5,    // Rotation limit on the z axis.
    ScaleX = 1 << 6,       // Scale limit on the x axis.
    ScaleY = 1 << 7,       // Scale limit on the y axis.
    ScaleZ = 1 << 8,       // Scale limit on the z axis.
}

enum XacAttribute {
    AttribPositions = 0, // Vertex positions. Typecast to MCore::Vector3. Positions are always exist.
    AttribNormals = 1,   // Vertex normals. Typecast to MCore::Vector3. Normals are always exist.
    AttribTangents = 2,  // Vertex tangents. Typecast to <b> MCore::Vector4 </b>.
    AttribUvcoords = 3,  // Vertex uv coordinates. Typecast to MCore::Vector2.
    AttribColors32 = 4,  // Vertex colors in 32-bits. Typecast to uint32.
    AttribOrgvtxnumbers = 5, // Original vertex numbers. Typecast to uint32. Original vertex numbers always exist.
    AttribColors128 = 6,     // Vertex colors in 128-bits. Typecast to MCore::RGBAColor.
    AttribBitangents = 7, // Vertex bitangents (aka binormal). Typecast to MCore::Vector3. When tangents exists bitangents may still not exist!
}

// collection of XAC chunk IDs
enum XacChunk {
    XacChunkNode = 0,
    XacChunkMesh = 1,
    XacChunkSkinninginfo = 2,
    XacChunkStdmaterial = 3,
    XacChunkStdmateriallayer = 4,
    XacChunkFxmaterial = 5,
    XacLimit = 6,
    XacChunkInfo = 7,
    XacChunkMeshlodlevels = 8,
    XacChunkStdprogmorphtarget = 9,
    XacChunkNodegroups = 10,
    XacChunkNodes = 11,             // XAC_Nodes
    XacChunkStdpmorphtargets = 12,  // XAC_PMorphTargets
    XacChunkMaterialinfo = 13,      // XAC_MaterialInfo
    XacChunkNodemotionsources = 14, // XAC_NodeMotionSources
    XacChunkAttachmentnodes = 15,   // XAC_AttachmentNodes
    XacForce32bit = 0xFFFFFFFF,
}

// material layer map types
enum XacMaterialLayer {
    XacLayeridUnknown = 0,       // unknown layer
    XacLayeridAmbient = 1,       // ambient layer
    XacLayeridDiffuse = 2,       // a diffuse layer
    XacLayeridSpecular = 3,      // specular layer
    XacLayeridOpacity = 4,       // opacity layer
    XacLayeridBump = 5,          // bump layer
    XacLayeridSelfillum = 6,     // self illumination layer
    XacLayeridShine = 7,         // shininess (for specular)
    XacLayeridShinestrength = 8, // shine strength (for specular)
    XacLayeridFiltercolor = 9,   // filter color layer
    XacLayeridReflect = 10,      // reflection layer
    XacLayeridRefract = 11,      // refraction layer
    XacLayeridEnvironment = 12,  // environment map layer
    XacLayeridDisplacement = 13, // displacement map layer
    XacLayeridForce8bit = 0xFF,  // don't use more than 8 bit values
}

#[derive(Debug, Serialize, Deserialize)]
enum XacChunkData {
    XacInfo(XacInfo),
    XacInfo2(XacInfo2),
    XacInfo3(XacInfo3),
    XacInfo4(XacInfo4),

    XacNode(XacNode),
    XacNode2(XacNode2),
    XacNode3(XacNode3),
    XacNode4(XacNode4),

    XacSkinningInfo(XacSkinningInfo),
    XacSkinningInfo2(XacSkinningInfo2),
    XacSkinningInfo3(XacSkinningInfo3),
    XacSkinningInfo4(XacSkinningInfo4),

    XacStandardMaterial(XacStandardMaterial),
    XacStandardMaterial2(XacStandardMaterial2),
    XacStandardMaterial3(XacStandardMaterial3),

    XACStandardMaterialLayer(XACStandardMaterialLayer),
    XACStandardMaterialLayer2(XACStandardMaterialLayer2),

    XACSubMesh(XACSubMesh),
    XACMesh(XACMesh),
    XACMesh2(XACMesh2),

    XACLimit(XACLimit),
    XACPMorphTarget(XACPMorphTarget),
    XACPMorphTargets(XACPMorphTargets),

    XACFXMaterial(XACFXMaterial),
    XACFXMaterial2(XACFXMaterial2),
    XACFXMaterial3(XACFXMaterial3),

    XACNodeGroup(XACNodeGroup),
    XACNodes(XACNodes),

    XACMaterialInfo(XACMaterialInfo),
    XACMaterialInfo2(XACMaterialInfo2),

    XACMeshLodLevel(XACMeshLodLevel),

    XACNodeMotionSources(XACNodeMotionSources),
    XACAttachmentNodes(XACAttachmentNodes),
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct FileChunk {
    chunk_id: u32,      // The chunk ID
    size_in_bytes: u32, // The size in bytes of this chunk (excluding this struct)
    version: u32,       // The version of the chunk
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)] // Color [0..1] range
struct FileColor {
    color_red: f32,   // Red
    color_green: f32, // Green
    color_blue: f32,  // Blue
    color_alpha: f32, // Alpha
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)] // A 3D vector
struct FileVector3 {
    axis_x: f32, // x+ = to the right
    axis_y: f32, // y+ = up
    axis_z: f32, // z+ = forwards (into the depth)
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)] // A compressed 3D vector
struct File16BitVector3 {
    axis_x: u16, // x+ = to the right
    axis_y: u16, // y+ = up
    axis_z: u16, // z+ = forwards (into the depth)
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)] // A compressed 3D vector
struct File8BitVector3 {
    axis_x: u8, // x+ = to the right
    axis_y: u8, // y+ = up
    axis_z: u8, // z+ = forwards (into the depth)
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)] // A quaternion
struct FileQuaternion {
    axis_x: f32,
    axis_y: f32,
    axis_z: f32,
    axis_w: f32,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)] // The 16-bit component quaternion
struct File16BitQuaternion {
    axis_x: i16,
    axis_y: i16,
    axis_z: i16,
    axis_w: i16,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacHeader {
    fourcc: u32,     // Must be "XAC "
    hi_version: u8,  // High version (e.g., 2 in v2.34)
    lo_version: u8,  // Low version (e.g., 34 in v2.34)
    endian_type: u8, // Endianness: 0 = little, 1 = big
    mul_order: u8,   // See enum MULORDER_...
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacInfo {
    repositioning_mask: u32,
    repositioning_node_index: u32,
    exporter_high_version: u8,
    exporter_low_version: u8,
    padding: u16,

    #[br(temp)]
    source_app_length: u32,
    #[br(count = source_app_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    source_app: String,

    #[br(temp)]
    original_filename_length: u32,
    #[br(count = original_filename_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    original_filename: String,

    #[br(temp)]
    compilation_date_length: u32,
    #[br(count = compilation_date_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    compilation_date: String,

    #[br(temp)]
    actor_name_length: u32,
    #[br(count = actor_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    actor_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacInfo2 {
    repositioning_mask: u32,
    repositioning_node_index: u32,
    exporter_high_version: u8,
    exporter_low_version: u8,
    retarget_root_offset: f32,
    padding: u16,

    #[br(temp)]
    source_app_length: u32,
    #[br(count = source_app_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    source_app: String,

    #[br(temp)]
    original_filename_length: u32,
    #[br(count = original_filename_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    original_filename: String,

    #[br(temp)]
    compilation_date_length: u32,
    #[br(count = compilation_date_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    compilation_date: String,

    #[br(temp)]
    actor_name_length: u32,
    #[br(count = actor_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    actor_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacInfo3 {
    trajectory_node_index: u32,
    motion_extraction_node_index: u32,
    motion_extraction_mask: u32,
    exporter_high_version: u8,
    exporter_low_version: u8,
    retarget_root_offset: f32,
    padding: u16,

    #[br(temp)]
    source_app_length: u32,
    #[br(count = source_app_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    source_app: String,

    #[br(temp)]
    original_filename_length: u32,
    #[br(count = original_filename_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    original_filename: String,

    #[br(temp)]
    compilation_date_length: u32,
    #[br(count = compilation_date_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    compilation_date: String,

    #[br(temp)]
    actor_name_length: u32,
    #[br(count = actor_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    actor_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacInfo4 {
    num_lods: u32,
    trajectory_node_index: u32,
    motion_extraction_node_index: u32,
    exporter_high_version: u8,
    exporter_low_version: u8,
    retarget_root_offset: f32,
    padding: u16,

    #[br(temp)]
    source_app_length: u32,
    #[br(count = source_app_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    source_app: String,

    #[br(temp)]
    original_filename_length: u32,
    #[br(count = original_filename_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    original_filename: String,

    #[br(temp)]
    compilation_date_length: u32,
    #[br(count = compilation_date_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    compilation_date: String,

    #[br(temp)]
    actor_name_length: u32,
    #[br(count = actor_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    actor_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacNode {
    local_quat: FileQuaternion,
    scale_rot: FileQuaternion,
    local_pos: FileVector3,
    local_scale: FileVector3,
    shear: FileVector3,
    skeletal_lods: u32,
    parent_index: u32,

    #[br(temp)]
    node_name_length: u32,
    #[br(count = node_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    node_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacNode2 {
    local_quat: FileQuaternion,
    scale_rot: FileQuaternion,
    local_pos: FileVector3,
    local_scale: FileVector3,
    shear: FileVector3,
    skeletal_lods: u32,
    parent_index: u32,
    node_flags: u8,
    padding: [u8; 3],

    #[br(temp)]
    node_name_length: u32,
    #[br(count = node_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    node_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacNode3 {
    local_quat: FileQuaternion,
    scale_rot: FileQuaternion,
    local_pos: FileVector3,
    local_scale: FileVector3,
    shear: FileVector3,
    skeletal_lods: u32,
    parent_index: u32,
    node_flags: u8,
    obb: [f32; 16], // Oriented Bounding Box (OBB)
    padding: [u8; 3],

    #[br(temp)]
    node_name_length: u32,
    #[br(count = node_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    node_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacNode4 {
    local_quat: FileQuaternion,
    scale_rot: FileQuaternion,
    local_pos: FileVector3,
    local_scale: FileVector3,
    shear: FileVector3,
    skeletal_lods: u32,
    motion_lods: u32,
    parent_index: u32,
    num_children: u32,
    node_flags: u8,
    obb: [f32; 16],         // Oriented Bounding Box (OBB)
    importance_factor: f32, // Used for automatic motion LOD
    padding: [u8; 3],

    #[br(temp)]
    node_name_length: u32,
    #[br(count = node_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    node_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACMeshLodLevel {
    lod_level: u32,
    size_in_bytes: u32,
    // Followed by:
    // Vec<u8> representing LOD model memory file
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacUv {
    axis_u: f32, // U texture coordinate
    axis_v: f32, // V texture coordinate
}

#[derive(Default, Debug, Serialize, Deserialize, BinRead)]
#[br(little)]
struct XacSkinningInfo {
    node_index: u32,
    is_for_collision_mesh: u8,
    padding: [u8; 3],
    // Fix this idk what is this mean!!!
    // Followed by:
    // for all mesh original num vertices
    //     num_influences: u8
    //         XacSkinInfluence[num_influences]
}

#[derive(Default, Debug, Serialize, Deserialize, BinRead)]
#[br(import(num_org_verts:u32))]
#[br(little)]
struct XacSkinningInfo2 {
    node_index: u32,           // The node number in the actor
    num_total_influences: u32, // Total number of influences of all vertices together
    is_for_collision_mesh: u8, // Is it for a collision mesh?
    padding: [u8; 3],

    #[br(count = num_total_influences)]
    skinning_influence: Vec<XacSkinInfluence>,

    #[br(count = num_org_verts)]
    skinning_info_table_entry: Vec<XacSkinningInfoTableEntry>,
}

#[derive(Default, Debug, Serialize, Deserialize, BinRead)]
#[br(import(num_org_verts:u32))]
#[br(little)]
struct XacSkinningInfo3 {
    node_index: u32,           // The node number in the actor
    num_local_bones: u32,      // Number of local bones used by the mesh
    num_total_influences: u32, // Total number of influences of all vertices together
    is_for_collision_mesh: u8, // Is it for a collision mesh?
    padding: [u8; 3],

    #[br(count = num_total_influences)]
    skinning_influence: Vec<XacSkinInfluence>,

    #[br(count = num_org_verts)]
    skinning_info_table_entry: Vec<XacSkinningInfoTableEntry>,
}

#[derive(Default, Debug, Serialize, Deserialize, BinRead)]
#[br(import(num_org_verts:u32))]
#[br(little)]
struct XacSkinningInfo4 {
    node_index: u32,           // The node number in the actor
    lod: u32,                  // Level of detail
    num_local_bones: u32,      // Number of local bones used by the mesh
    num_total_influences: u32, // Total number of influences of all vertices together
    is_for_collision_mesh: u8, // Is it for a collision mesh?
    padding: [u8; 3],

    #[br(count = num_total_influences)]
    skinning_influence: Vec<XacSkinInfluence>,

    #[br(count = num_org_verts)]
    skinning_info_table_entry: Vec<XacSkinningInfoTableEntry>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacSkinningInfoTableEntry {
    start_index: u32,  // Index inside the SkinInfluence array
    num_elements: u32, // Number of influences for this item/entry
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacSkinInfluence {
    weight: f32,
    node_number: u32,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacStandardMaterial {
    ambient: FileColor,    // Ambient color
    diffuse: FileColor,    // Diffuse color
    specular: FileColor,   // Specular color
    emissive: FileColor,   // Self-illumination color
    shine: f32,            // Shine
    shine_strength: f32,   // Shine strength
    opacity: f32,          // Opacity (1.0 = full opaque, 0.0 = full transparent)
    ior: f32,              // Index of refraction
    double_sided: u8,      // Double-sided?
    wireframe: u8,         // Render in wireframe?
    transparency_type: u8, // F=filter / S=subtractive / A=additive / U=unknown
    padding: u8,

    #[br(temp)]
    material_name_length: u32,
    #[br(count = material_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    material_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacStandardMaterial2 {
    ambient: FileColor,
    diffuse: FileColor,
    specular: FileColor,
    emissive: FileColor,
    shine: f32,
    shine_strength: f32,
    opacity: f32,
    ior: f32,
    double_sided: u8,
    wireframe: u8,
    transparency_type: u8,
    num_layers: u8, // Number of material layers

    #[br(temp)]
    material_name_length: u32,
    #[br(count = material_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    material_name: String,
    #[br(count = num_layers)]
    standard_material_layer2: Vec<XACStandardMaterialLayer2>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XacStandardMaterial3 {
    lod: u32, // Level of detail
    ambient: FileColor,
    diffuse: FileColor,
    specular: FileColor,
    emissive: FileColor,
    shine: f32,
    shine_strength: f32,
    opacity: f32,
    ior: f32,
    double_sided: u8,
    wireframe: u8,
    transparency_type: u8,
    num_layers: u8, // Number of material layers

    #[br(temp)]
    material_name_length: u32,
    #[br(count = material_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    material_name: String,
    #[br(count = num_layers)]
    standard_material_layer2: Vec<XACStandardMaterialLayer2>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACStandardMaterialLayer {
    amount: f32,           // the amount, between 0 and 1
    u_offset: f32,         // u offset (horizontal texture shift)
    v_offset: f32,         // v offset (vertical texture shift)
    u_tiling: f32,         // horizontal tiling factor
    v_tiling: f32,         // vertical tiling factor
    rotation_radians: f32, // texture rotation in radians
    material_number: u16,  // the parent material number (0 means first material)
    map_type: u8,          // the map type
    padding: u8,           // alignment
    #[br(temp)]
    texture_name_length: u32,
    #[br(count = texture_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    texture_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACStandardMaterialLayer2 {
    amount: f32,
    u_offset: f32,
    v_offset: f32,
    u_tiling: f32,
    v_tiling: f32,
    rotation_radians: f32,
    material_number: u16,
    map_type: u8,
    blend_mode: u8, // blend mode for texture layering
    #[br(temp)]
    texture_name_length: u32,
    #[br(count = texture_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    texture_name: String,
}

#[derive(Default, Debug, Serialize, Deserialize, BinRead)]
#[br(import(total_verts:u32))]
#[br(little)]
struct XACVertexAttributeLayer {
    layer_type_id: u32,
    attrib_size_in_bytes: u32,
    enable_deformations: u8,
    is_scale: u8,
    padding: [u8; 2],

    #[br(count = attrib_size_in_bytes * total_verts )]
    mesh_data: Vec<u8>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACSubMesh {
    num_indices: u32,
    num_verts: u32,
    material_index: u32,
    num_bones: u32,

    #[br(count = num_indices)]
    indices: Vec<u32>,

    #[br(count = num_bones)]
    bones: Vec<u32>,
}

#[derive(Default, Debug, Serialize, Deserialize, BinRead)]
#[br(little)]
struct XACMesh {
    node_index: u32,
    num_org_verts: u32,
    total_verts: u32,
    total_indices: u32,
    num_sub_meshes: u32,
    num_layers: u32,
    is_collision_mesh: u8,
    padding: [u8; 3],

    #[br(args { inner: (total_verts,) })]
    #[br(count = num_layers)]
    vertex_attribute_layer: Vec<XACVertexAttributeLayer>,
    #[br(count = num_sub_meshes)]
    sub_meshes: Vec<XACSubMesh>,
}

#[derive(Default, Debug, Serialize, Deserialize, BinRead)]
#[br(little)]
struct XACMesh2 {
    node_index: u32,
    lod: u32,
    num_org_verts: u32,
    total_verts: u32,
    total_indices: u32,
    num_sub_meshes: u32,
    num_layers: u32,
    is_collision_mesh: u8,
    padding: [u8; 3],

    #[br(args { inner: (total_verts,) })]
    #[br(count = num_layers)]
    vertex_attribute_layer: Vec<XACVertexAttributeLayer>,
    #[br(count = num_sub_meshes)]
    sub_meshes: Vec<XACSubMesh>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACLimit {
    translation_min: FileVector3,
    translation_max: FileVector3,
    rotation_min: FileVector3,
    rotation_max: FileVector3,
    scale_min: FileVector3,
    scale_max: FileVector3,
    limit_flags: [u8; 9], // limit type activation flags
    node_number: u32,     // the node number where this info belongs
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACPMorphTarget {
    range_min: f32,              // the slider min
    range_max: f32,              // the slider max
    lod: u32,                    // LOD level
    num_mesh_deform_deltas: u32, // number of mesh deform data objects
    num_transformations: u32,    // number of transformations
    phoneme_sets: u32,           // number of phoneme sets

    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,
    #[br(count = num_mesh_deform_deltas)]
    morph_target_mesh_deltas: Vec<XACPMorphTargetMeshDeltas>,
    #[br(count = num_transformations)]
    morph_target_transform: Vec<XACPMorphTargetTransform>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACPMorphTargets {
    num_morph_targets: u32, // number of morph targets
    lod: u32,               // LOD level
    #[br(count = num_morph_targets)]
    morph_targets: Vec<XACPMorphTargets>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACPMorphTargetMeshDeltas {
    node_index: u32,
    min_value: f32,    // min range for x, y, z of compressed position vectors
    max_value: f32,    // max range for x, y, z of compressed position vectors
    num_vertices: u32, // number of deltas
    #[br(count = num_vertices)]
    delta_position_values: Vec<File16BitVector3>,
    #[br(count = num_vertices)]
    delta_normal_values: Vec<File8BitVector3>,
    #[br(count = num_vertices)]
    delta_tangent_values: Vec<File8BitVector3>,
    #[br(count = num_vertices)]
    vertex_numbers: Vec<u32>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACPMorphTargetTransform {
    node_index: u32,                // node name where transform belongs
    rotation: FileQuaternion,       // node rotation
    scale_rotation: FileQuaternion, // node delta scale rotation
    position: FileVector3,          // node delta position
    scale: FileVector3,             // node delta scale
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACFXMaterial {
    num_int_params: u32,
    num_float_params: u32,
    num_color_params: u32,
    num_bitmap_params: u32,
    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,
    #[br(temp)]
    effect_file_length: u32,
    #[br(count = effect_file_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    effect_file: String,
    #[br(temp)]
    shader_technique_length: u32,
    #[br(count = shader_technique_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    shader_technique: String,

    #[br(if(num_int_params > 0), count = num_int_params)]
    xac_fx_int_parameter: Option<Vec<XACFXIntParameter>>,

    #[br(if(num_float_params > 0), count = num_float_params)]
    xac_fx_float_parameter: Option<Vec<XACFXFloatParameter>>,

    #[br(if(num_color_params > 0), count = num_color_params)]
    xac_fx_color_parameter: Option<Vec<XACFXColorParameter>>,

    #[br(if(num_bitmap_params > 0), count = num_bitmap_params)]
    xac_fx_bitmap_parameter: Option<Vec<XACFXBitmapParameter>>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACFXMaterial2 {
    num_int_params: u32,
    num_float_params: u32,
    num_color_params: u32,
    num_bool_params: u32,
    num_vector3_params: u32,
    num_bitmap_params: u32,
    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,
    #[br(temp)]
    effect_file_length: u32,
    #[br(count = effect_file_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    effect_file: String,
    #[br(temp)]
    shader_technique_length: u32,
    #[br(count = shader_technique_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    shader_technique: String,

    #[br(if(num_int_params > 0), count = num_int_params)]
    xac_fx_int_parameter: Option<Vec<XACFXIntParameter>>,

    #[br(if(num_float_params > 0), count = num_float_params)]
    xac_fx_float_parameter: Option<Vec<XACFXFloatParameter>>,

    #[br(if(num_color_params > 0), count = num_color_params)]
    xac_fx_color_parameter: Option<Vec<XACFXColorParameter>>,

    #[br(if(num_bool_params > 0), count = num_bool_params)]
    xac_fx_bool_parameter: Option<Vec<XACFXBoolParameter>>,

    #[br(if(num_vector3_params > 0), count = num_vector3_params)]
    xac_fx_vector3_parameter: Option<Vec<XACFXVector3Parameter>>,

    #[br(if(num_bitmap_params > 0), count = num_bitmap_params)]
    xac_fx_bitmap_parameter: Option<Vec<XACFXBitmapParameter>>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACFXMaterial3 {
    lod: u32, // level of detail
    num_int_params: u32,
    num_float_params: u32,
    num_color_params: u32,
    num_bool_params: u32,
    num_vector3_params: u32,
    num_bitmap_params: u32,
    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,
    #[br(temp)]
    effect_file_length: u32,
    #[br(count = effect_file_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    effect_file: String,
    #[br(temp)]
    shader_technique_length: u32,
    #[br(count = shader_technique_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    shader_technique: String,

    #[br(if(num_int_params > 0), count = num_int_params)]
    xac_fx_int_parameter: Option<Vec<XACFXIntParameter>>,

    #[br(if(num_float_params > 0), count = num_float_params)]
    xac_fx_float_parameter: Option<Vec<XACFXFloatParameter>>,

    #[br(if(num_color_params > 0), count = num_color_params)]
    xac_fx_color_parameter: Option<Vec<XACFXColorParameter>>,

    #[br(if(num_bool_params > 0), count = num_bool_params)]
    xac_fx_bool_parameter: Option<Vec<XACFXBoolParameter>>,

    #[br(if(num_vector3_params > 0), count = num_vector3_params)]
    xac_fx_vector3_parameter: Option<Vec<XACFXVector3Parameter>>,

    #[br(if(num_bitmap_params > 0), count = num_bitmap_params)]
    xac_fx_bitmap_parameter: Option<Vec<XACFXBitmapParameter>>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACFXIntParameter {
    value: i32, // Beware, signed integer since negative values are allowed
    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACFXFloatParameter {
    value: f32,
    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACFXColorParameter {
    value: FileColor,
    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACFXVector3Parameter {
    value: FileVector3,
    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACFXBoolParameter {
    value: u8, // 0 = no, 1 = yes
    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACFXBitmapParameter {
    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,

    #[br(temp)]
    value_name_length: u32,
    #[br(count = value_name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    value_name: String,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACNodeGroup {
    num_nodes: u16,
    disabled_on_default: u8, // 0 = no, 1 = yes

    #[br(temp)]
    name_length: u32,
    #[br(count = name_length, map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    name: String,

    #[br(count = num_nodes)]
    data: Vec<u16>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACNodes {
    num_nodes: u32,
    num_root_nodes: u32,

    #[br(count = num_nodes)]
    xac_node: Vec<XacNode4>,
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACMaterialInfo {
    num_total_materials: u32, // Total number of materials to follow (including default/extra material)
    num_standard_materials: u32, // Number of standard materials in the file
    num_fx_materials: u32,    // Number of FX materials in the file
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACMaterialInfo2 {
    lod: u32,                    // Level of detail
    num_total_materials: u32, // Total number of materials to follow (including default/extra material)
    num_standard_materials: u32, // Number of standard materials in the file
    num_fx_materials: u32,    // Number of FX materials in the file
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACNodeMotionSources {
    num_nodes: u32,

    #[br(count = num_nodes)]
    node_indices: Vec<u16>, // List of node indices (optional if mirroring is not set)
}

#[binread]
#[derive(Default, Debug, Serialize, Deserialize)]
#[br(little)]
struct XACAttachmentNodes {
    num_nodes: u32,

    #[br(count = num_nodes)]
    attachment_indices: Vec<u16>, // List of node indices for attachments
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct XACFile {
    header: XacHeader,
    chunk: Vec<FileChunk>,
    chunk_data: Vec<XacChunkData>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct SubMesh {
    pub texture_name: String,
    pub position_count: usize,
    pub positions: Vec<[f32; 3]>,
    pub normal_count: usize,
    pub normals: Vec<[f32; 3]>,
    pub tangent_count: usize,
    pub tangents: Vec<[f32; 4]>,
    pub uvcoord_count: usize,
    pub uvcoords: Vec<[f32; 2]>,
    pub color32_count: usize,
    pub colors32: Vec<u32>,
    pub original_vertex_numbers_count: usize,
    pub original_vertex_numbers: Vec<u32>,
    pub color128_count: usize,
    pub colors128: Vec<[f32; 4]>,
    pub bitangent_count: usize,
    pub bitangents: Vec<[f32; 3]>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Mesh {
    pub submesh_count: usize,
    pub submeshes: Vec<SubMesh>,
}

impl XACFile {
    pub fn load_from_file<P: AsRef<Path>>(file_path: P) -> io::Result<Self> {
        let file = std::fs::File::open(file_path)?;
        let mut buf_reader = BufReader::new(file);
        let mut binary_reader = BinaryReader::new(&mut buf_reader);
        Self::load_from_reader(&mut binary_reader)
    }

    pub fn load_from_bytes(mut bytes: Vec<u8>) -> io::Result<Self> {
        let cursor = Cursor::new(&mut bytes);
        let mut binary_reader = BinaryReader::new(cursor);
        Self::load_from_reader(&mut binary_reader)
    }

    fn load_from_reader<R: Read + Seek>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        let mut xac_data = XACFile::default();
        xac_data.read_header(reader)?;
        xac_data.read_chunk(reader)?;

        Ok(xac_data)
    }

    fn read_header<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> io::Result<&mut Self> {
        self.header = XacHeader::read(&mut reader.reader).unwrap(); // Use binread to read the struct
        Ok(self)
    }

    fn read_chunk<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> io::Result<&mut Self> {
        while !reader.is_eof()? {
            // Read chunk header: chunk_id, size_in_bytes, and version
            let chunk = FileChunk {
                chunk_id: reader.read_u32()?,
                size_in_bytes: reader.read_u32()?,
                version: reader.read_u32()?,
            };

            // Get the current position before processing the chunk
            let position = reader.tell()?;

            // Process the chunk (pass the reference to the chunk and reader)
            self.process_chunk(&chunk, reader);

            // Calculate the target position after the chunk is fully read
            let target_pos = position + chunk.size_in_bytes as u64;

            // Check if the current position matches the target position
            if target_pos != reader.tell().unwrap() {
                let missing_bytes = target_pos as i64 - reader.tell().unwrap() as i64;
                println!(
                    "Need {} more bytes to finish this chunk id : {}",
                    missing_bytes, chunk.chunk_id
                );
            }

            // Seek to the target position after the chunk has been processed
            reader.seek(SeekFrom::Start(target_pos))?;

            // Push the processed chunk into the chunk vector
            self.chunk.push(chunk);
        }

        Ok(self)
    }

    fn process_chunk<R: Read + Seek>(&mut self, chunk: &FileChunk, reader: &mut BinaryReader<R>) {
        match chunk.chunk_id {
            id if id == XacChunk::XacChunkNode as u32 => {
                println!(
                    "Chunk: XacChunkNode, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let node = match chunk.version {
                    1 => Some(XacChunkData::XacNode(self.read_xac_node(reader))),
                    2 => Some(XacChunkData::XacNode2(self.read_xac_node2(reader))),
                    3 => Some(XacChunkData::XacNode3(self.read_xac_node3(reader))),
                    4 => Some(XacChunkData::XacNode4(self.read_xac_node4(reader))),
                    _ => None,
                };
                if let Some(data) = node {
                    self.chunk_data.push(data);
                } else {
                    println!("Unknown version {} for XacChunkNode", chunk.version);
                }
            }
            id if id == XacChunk::XacChunkMesh as u32 => {
                println!(
                    "Chunk: XacChunkMesh, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let mesh = match chunk.version {
                    1 => Some(XacChunkData::XACMesh(self.read_xac_mesh(reader))),
                    2 => Some(XacChunkData::XACMesh2(self.read_xac_mesh2(reader))),
                    _ => None,
                };
                if let Some(data) = mesh {
                    self.chunk_data.push(data);
                } else {
                    println!("Unknown version {} for XacChunkMesh", chunk.version);
                }
            }
            id if id == XacChunk::XacChunkSkinninginfo as u32 => {
                println!(
                    "Chunk: XacChunkSkinninginfo, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let skinning_info = match chunk.version {
                    1 => Some(XacChunkData::XacSkinningInfo(
                        self.read_xac_skinning_info(reader),
                    )),
                    2 => Some(XacChunkData::XacSkinningInfo2(
                        self.read_xac_skinning_info2(reader),
                    )),
                    3 => Some(XacChunkData::XacSkinningInfo3(
                        self.read_xac_skinning_info3(reader),
                    )),
                    4 => Some(XacChunkData::XacSkinningInfo4(
                        self.read_xac_skinning_info4(reader),
                    )),
                    _ => None,
                };
                if let Some(data) = skinning_info {
                    self.chunk_data.push(data);
                } else {
                    println!("Unknown version {} for XacChunkSkinninginfo", chunk.version);
                }
            }
            id if id == XacChunk::XacChunkStdmaterial as u32 => {
                println!(
                    "Chunk: XacChunkStdmaterial, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let material = match chunk.version {
                    1 => Some(XacChunkData::XacStandardMaterial(
                        self.read_xac_standard_material(reader),
                    )),
                    2 => Some(XacChunkData::XacStandardMaterial2(
                        self.read_xac_standard_material2(reader),
                    )),
                    3 => Some(XacChunkData::XacStandardMaterial3(
                        self.read_xac_standard_material3(reader),
                    )),
                    _ => None,
                };
                if let Some(data) = material {
                    self.chunk_data.push(data);
                } else {
                    println!("Unknown version {} for XacChunkStdmaterial", chunk.version);
                }
            }
            id if id == XacChunk::XacChunkStdmateriallayer as u32 => {
                println!(
                    "Chunk: XacChunkStdmateriallayer, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let material_layer = match chunk.version {
                    1 => Some(XacChunkData::XACStandardMaterialLayer(
                        self.read_xac_standard_material_layer(reader),
                    )),
                    2 => Some(XacChunkData::XACStandardMaterialLayer2(
                        self.read_xac_standard_material_layer2(reader),
                    )),
                    _ => None,
                };
                if let Some(data) = material_layer {
                    self.chunk_data.push(data);
                } else {
                    println!(
                        "Unknown version {} for XacChunkStdmateriallayer",
                        chunk.version
                    );
                }
            }
            id if id == XacChunk::XacChunkFxmaterial as u32 => {
                println!(
                    "Chunk: XacChunkFxmaterial, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let fx_material = match chunk.version {
                    1 => Some(XacChunkData::XACFXMaterial(
                        self.read_xac_fx_material(reader),
                    )),
                    2 => Some(XacChunkData::XACFXMaterial2(
                        self.read_xac_fx_material2(reader),
                    )),
                    3 => Some(XacChunkData::XACFXMaterial3(
                        self.read_xac_fx_material3(reader),
                    )),
                    _ => None,
                };
                if let Some(data) = fx_material {
                    self.chunk_data.push(data);
                } else {
                    println!("Unknown version {} for XacChunkFxmaterial", chunk.version);
                }
            }
            id if id == XacChunk::XacChunkMaterialinfo as u32 => {
                println!(
                    "Chunk: XacChunkMaterialinfo, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let material_info = match chunk.version {
                    1 => Some(XacChunkData::XACMaterialInfo(
                        self.read_xac_material_info(reader),
                    )),
                    2 => Some(XacChunkData::XACMaterialInfo2(
                        self.read_xac_material_info2(reader),
                    )),
                    _ => None,
                };
                if let Some(data) = material_info {
                    self.chunk_data.push(data);
                } else {
                    println!("Unknown version {} for XacChunkMaterialinfo", chunk.version);
                }
            }
            id if id == XacChunk::XacChunkNodes as u32 => {
                println!(
                    "Chunk: XacChunkNodes, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let nodes = match chunk.version {
                    1 => Some(XacChunkData::XACNodes(self.read_xac_nodes(reader))),
                    _ => None,
                };
                if let Some(data) = nodes {
                    self.chunk_data.push(data);
                } else {
                    println!("Unknown version {} for XacChunkNodes", chunk.version);
                }
            }
            id if id == XacChunk::XacChunkNodegroups as u32 => {
                println!(
                    "Chunk: XacChunkNodegroups, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let node_group = match chunk.version {
                    1 => Some(XacChunkData::XACNodeGroup(self.read_xac_node_group(reader))),
                    _ => None,
                };
                if let Some(data) = node_group {
                    self.chunk_data.push(data);
                } else {
                    println!("Unknown version {} for XacChunkNodegroups", chunk.version);
                }
            }
            id if id == XacChunk::XacChunkMeshlodlevels as u32 => {
                println!(
                    "Chunk: XacChunkMeshlodlevels, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let mesh_lod = match chunk.version {
                    1 => Some(XacChunkData::XACMeshLodLevel(
                        self.read_xac_mesh_lod_level(reader),
                    )),
                    _ => None,
                };
                if let Some(data) = mesh_lod {
                    self.chunk_data.push(data);
                } else {
                    println!(
                        "Unknown version {} for XacChunkMeshlodlevels",
                        chunk.version
                    );
                }
            }
            id if id == XacChunk::XacLimit as u32 => {
                println!(
                    "Chunk: XacLimit, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let mesh_lod = match chunk.version {
                    1 => Some(XacChunkData::XACLimit(self.read_xac_limit(reader))),
                    _ => None,
                };
                if let Some(data) = mesh_lod {
                    self.chunk_data.push(data);
                } else {
                    println!("Unknown version {} for XacLimit", chunk.version);
                }
            }
            id if id == XacChunk::XacChunkInfo as u32 => {
                println!(
                    "Chunk: XacChunkInfo, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let mesh_lod = match chunk.version {
                    1 => Some(XacChunkData::XacInfo(self.read_xac_info(reader))),
                    2 => Some(XacChunkData::XacInfo2(self.read_xac_info2(reader))),
                    3 => Some(XacChunkData::XacInfo3(self.read_xac_info3(reader))),
                    4 => Some(XacChunkData::XacInfo4(self.read_xac_info4(reader))),
                    _ => None,
                };
                if let Some(data) = mesh_lod {
                    self.chunk_data.push(data);
                } else {
                    println!("Unknown version {} for XacChunkInfo", chunk.version);
                }
            }
            id if id == XacChunk::XacChunkStdprogmorphtarget as u32 => {
                println!(
                    "Chunk: XacChunkStdprogmorphtarget, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let mesh_lod = match chunk.version {
                    1 => Some(XacChunkData::XACPMorphTarget(
                        self.read_xac_pmorph_target(reader),
                    )),
                    _ => None,
                };
                if let Some(data) = mesh_lod {
                    self.chunk_data.push(data);
                } else {
                    println!(
                        "Unknown version {} for XacChunkStdprogmorphtarget",
                        chunk.version
                    );
                }
            }

            id if id == XacChunk::XacChunkStdpmorphtargets as u32 => {
                println!(
                    "Chunk: XacChunkStdpmorphtargets, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let mesh_lod = match chunk.version {
                    1 => Some(XacChunkData::XACPMorphTargets(
                        self.read_xac_pmorph_targets(reader),
                    )),
                    _ => None,
                };
                if let Some(data) = mesh_lod {
                    self.chunk_data.push(data);
                } else {
                    println!(
                        "Unknown version {} for XacChunkStdpmorphtargets",
                        chunk.version
                    );
                }
            }

            id if id == XacChunk::XacChunkNodemotionsources as u32 => {
                println!(
                    "Chunk: XacChunkNodemotionsources, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let mesh_lod = match chunk.version {
                    1 => Some(XacChunkData::XACNodeMotionSources(
                        self.read_xac_node_motion_sources(reader),
                    )),
                    _ => None,
                };
                if let Some(data) = mesh_lod {
                    self.chunk_data.push(data);
                } else {
                    println!(
                        "Unknown version {} for XacChunkNodemotionsources",
                        chunk.version
                    );
                }
            }

            id if id == XacChunk::XacChunkAttachmentnodes as u32 => {
                println!(
                    "Chunk: XacChunkAttachmentnodes, Size: {}, Version: {}",
                    chunk.size_in_bytes, chunk.version
                );
                let mesh_lod = match chunk.version {
                    1 => Some(XacChunkData::XACAttachmentNodes(
                        self.read_xac_attachment_nodes(reader),
                    )),
                    _ => None,
                };
                if let Some(data) = mesh_lod {
                    self.chunk_data.push(data);
                } else {
                    println!(
                        "Unknown version {} for XacChunkAttachmentnodes",
                        chunk.version
                    );
                }
            }
            _ => {
                println!(
                    "Unknown Chunk ID: {}, Size: {}, Version: {}",
                    chunk.chunk_id, chunk.size_in_bytes, chunk.version
                );
            }
        }
    }

    fn read_xac_info<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XacInfo {
        // Implement parsing logic
        XacInfo::read(&mut reader.reader).unwrap()
    }

    fn read_xac_info2<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XacInfo2 {
        XacInfo2::read(&mut reader.reader).unwrap()
    }

    fn read_xac_info3<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XacInfo3 {
        XacInfo3::read(&mut reader.reader).unwrap()
    }

    fn read_xac_info4<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XacInfo4 {
        XacInfo4::read(&mut reader.reader).unwrap()
    }

    fn read_xac_node<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XacNode {
        XacNode::read(&mut reader.reader).unwrap()
    }

    fn read_xac_node2<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XacNode2 {
        XacNode2::read(&mut reader.reader).unwrap()
    }

    fn read_xac_node3<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XacNode3 {
        XacNode3::read(&mut reader.reader).unwrap()
    }

    fn read_xac_node4<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XacNode4 {
        XacNode4::read(&mut reader.reader).unwrap()
    }

    fn read_xac_skinning_info<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XacSkinningInfo {
        XacSkinningInfo::read(&mut reader.reader).unwrap()
    }

    fn read_xac_skinning_info2<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XacSkinningInfo2 {
        let mut num_org_verts: u32 = 0;
        // Read node_index first and check for matches
        let node_id = reader.read_u32().unwrap(); // Read node_id once
        // Loop through the chunk_data to find the right chunk based on node_id
        for chunk in &self.chunk_data {
            match chunk {
                // Match the specific variant and check if node_id matches the read value
                XacChunkData::XACMesh(data) => {
                    if data.node_index == node_id {
                        // Set num_org_verts based on the matched chunk
                        num_org_verts = data.num_org_verts;
                        // Move back 4 bytes since we've already read the node_id
                        reader.skip_bytes(-4).unwrap();
                    }
                }
                XacChunkData::XACMesh2(data) => {
                    if data.node_index == node_id {
                        // Set num_org_verts based on the matched chunk
                        num_org_verts = data.num_org_verts;
                        // Move back 4 bytes since we've already read the node_id
                        reader.skip_bytes(-4).unwrap();
                    }
                }
                // Exhaustive match for other variants (to avoid non-exhaustive match warnings)
                _ => {
                    // Optionally, you can log or do something else for unmatched variants
                    // println!("Ignoring variant: {:?}", chunk);
                }
            }
        }
        XacSkinningInfo2::read_args(&mut reader.reader, (num_org_verts,)).unwrap()

        // Now that num_org_verts is set, read the XacSkinningInfo2 struct
    }

    fn read_xac_skinning_info3<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XacSkinningInfo3 {
        let mut num_org_verts: u32 = 0;
        // Read node_index first and check for matches
        let node_id = reader.read_u32().unwrap(); // Read node_id once
        // Loop through the chunk_data to find the right chunk based on node_id
        for chunk in &self.chunk_data {
            match chunk {
                // Match the specific variant and check if node_id matches the read value
                XacChunkData::XACMesh(data) => {
                    if data.node_index == node_id {
                        // Set num_org_verts based on the matched chunk
                        num_org_verts = data.num_org_verts;
                        // Move back 4 bytes since we've already read the node_id
                        reader.skip_bytes(-4).unwrap();
                    }
                }
                XacChunkData::XACMesh2(data) => {
                    if data.node_index == node_id {
                        // Set num_org_verts based on the matched chunk
                        num_org_verts = data.num_org_verts;
                        // Move back 4 bytes since we've already read the node_id
                        reader.skip_bytes(-4).unwrap();
                    }
                }
                // Exhaustive match for other variants (to avoid non-exhaustive match warnings)
                _ => {
                    // Optionally, you can log or do something else for unmatched variants
                    // println!("Ignoring variant: {:?}", chunk);
                }
            }
        }
        XacSkinningInfo3::read_args(&mut reader.reader, (num_org_verts,)).unwrap()
    }

    fn read_xac_skinning_info4<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XacSkinningInfo4 {
        let mut num_org_verts: u32 = 0;
        // Read node_index first and check for matches
        let node_id = reader.read_u32().unwrap(); // Read node_id once
        // Loop through the chunk_data to find the right chunk based on node_id
        for chunk in &self.chunk_data {
            match chunk {
                // Match the specific variant and check if node_id matches the read value
                XacChunkData::XACMesh(data) => {
                    if data.node_index == node_id {
                        // Set num_org_verts based on the matched chunk
                        num_org_verts = data.num_org_verts;
                        // Move back 4 bytes since we've already read the node_id
                        reader.skip_bytes(-4).unwrap();
                    }
                }
                XacChunkData::XACMesh2(data) => {
                    if data.node_index == node_id {
                        // Set num_org_verts based on the matched chunk
                        num_org_verts = data.num_org_verts;
                        // Move back 4 bytes since we've already read the node_id
                        reader.skip_bytes(-4).unwrap();
                    }
                }
                // Exhaustive match for other variants (to avoid non-exhaustive match warnings)
                _ => {
                    // Optionally, you can log or do something else for unmatched variants
                    // println!("Ignoring variant: {:?}", chunk);
                }
            }
        }
        XacSkinningInfo4::read_args(&mut reader.reader, (num_org_verts,)).unwrap()
    }

    fn read_xac_standard_material<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XacStandardMaterial {
        XacStandardMaterial::read(&mut reader.reader).unwrap()
    }

    fn read_xac_standard_material2<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XacStandardMaterial2 {
        XacStandardMaterial2::read(&mut reader.reader).unwrap()
    }

    fn read_xac_standard_material3<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XacStandardMaterial3 {
        XacStandardMaterial3::read(&mut reader.reader).unwrap()
    }

    fn read_xac_standard_material_layer<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACStandardMaterialLayer {
        XACStandardMaterialLayer::read(&mut reader.reader).unwrap()
    }

    fn read_xac_standard_material_layer2<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACStandardMaterialLayer2 {
        XACStandardMaterialLayer2::read(&mut reader.reader).unwrap()
    }

    fn read_xac_sub_mesh<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XACSubMesh {
        XACSubMesh::read(&mut reader.reader).unwrap()
    }

    fn read_xac_mesh<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XACMesh {
        XACMesh::read(&mut reader.reader).unwrap()
    }

    fn read_xac_mesh2<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XACMesh2 {
        XACMesh2::read(&mut reader.reader).unwrap()
    }

    fn read_xac_limit<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XACLimit {
        XACLimit::read(&mut reader.reader).unwrap()
    }

    fn read_xac_pmorph_target<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACPMorphTarget {
        XACPMorphTarget::read(&mut reader.reader).unwrap()
    }

    fn read_xac_pmorph_targets<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACPMorphTargets {
        XACPMorphTargets::read(&mut reader.reader).unwrap()
    }

    fn read_xac_fx_material<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACFXMaterial {
        XACFXMaterial::read(&mut reader.reader).unwrap()
    }

    fn read_xac_fx_material2<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACFXMaterial2 {
        XACFXMaterial2::read(&mut reader.reader).unwrap()
    }

    fn read_xac_fx_material3<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACFXMaterial3 {
        XACFXMaterial3::read(&mut reader.reader).unwrap()
    }

    fn read_xac_node_group<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACNodeGroup {
        XACNodeGroup::read(&mut reader.reader).unwrap()
    }

    fn read_xac_nodes<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> XACNodes {
        XACNodes::read(&mut reader.reader).unwrap()
    }

    fn read_xac_material_info<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACMaterialInfo {
        XACMaterialInfo::read(&mut reader.reader).unwrap()
    }

    fn read_xac_material_info2<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACMaterialInfo2 {
        XACMaterialInfo2::read(&mut reader.reader).unwrap()
    }

    fn read_xac_mesh_lod_level<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACMeshLodLevel {
        XACMeshLodLevel::read(&mut reader.reader).unwrap()
    }

    fn read_xac_node_motion_sources<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACNodeMotionSources {
        XACNodeMotionSources::read(&mut reader.reader).unwrap()
    }

    fn read_xac_attachment_nodes<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> XACAttachmentNodes {
        XACAttachmentNodes::read(&mut reader.reader).unwrap()
    }

    fn get_texture_names(&self) -> Vec<String> {
        let mut textures = Vec::new();

        for chunk in &self.chunk_data {
            match chunk {
                XacChunkData::XacStandardMaterial(material) => {
                    textures.push(material.material_name.clone());
                }
                XacChunkData::XacStandardMaterial2(material) => {
                    textures.push(material.material_name.clone());
                }
                XacChunkData::XacStandardMaterial3(material) => {
                    textures.push(material.material_name.clone());
                }
                XacChunkData::XACFXMaterial(material) => {
                    if let Some(bitmap_params) = &material.xac_fx_bitmap_parameter {
                        for bitmap in bitmap_params {
                            textures.push(bitmap.value_name.clone());
                        }
                    }
                }
                XacChunkData::XACFXMaterial2(material) => {
                    if let Some(bitmap_params) = &material.xac_fx_bitmap_parameter {
                        for bitmap in bitmap_params {
                            textures.push(bitmap.value_name.clone());
                        }
                    }
                }
                XacChunkData::XACFXMaterial3(material) => {
                    if let Some(bitmap_params) = &material.xac_fx_bitmap_parameter {
                        for bitmap in bitmap_params {
                            textures.push(bitmap.value_name.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        textures
    }

    pub fn export_all_meshes(&self, output_prefix: &str) -> io::Result<()> {
        for (i, chunk) in self.chunk_data.iter().enumerate() {
            match chunk {
                XacChunkData::XACMesh(mesh) => {
                    let filename = format!("{}_mesh_{}", output_prefix, i);
                    self.export_to_obj(mesh, &filename)?;
                }
                XacChunkData::XACMesh2(mesh) => {
                    let filename = format!("{}_mesh_{}", output_prefix, i);
                    self.export_to_obj2(mesh, &filename)?;
                }
                _ => continue,
            }
        }
        Ok(())
    }

    pub fn export_all_meshes_into_struct(&mut self) -> io::Result<Vec<Mesh>> {
        let mut all_meshes: Vec<Mesh> = Vec::new(); // Assuming Mesh is a struct and can be initialized with default values

        for (i, chunk) in self.chunk_data.iter().enumerate() {
            match chunk {
                XacChunkData::XACMesh(mesh) => {
                    // Directly move the mesh from chunk
                    all_meshes.push(self.export_to_struct(mesh)?); // Move the mesh
                }
                XacChunkData::XACMesh2(mesh) => {
                    // Directly move the mesh from chunk
                    all_meshes.push(self.export_to_struct2(mesh)?); // Move the mesh
                }
                _ => continue,
            }
        }

        Ok(all_meshes) // Return the final mesh after all iterations
    }

    fn export_to_obj(&self, mesh: &XACMesh, output_prefix: &str) -> io::Result<()> {
        let texture_name = self.get_texture_names();

        let positions_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribPositions as u32);

        let normals_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribNormals as u32);

        let uvs_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribUvcoords as u32);

        if positions_layer.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No vertex positions found",
            ));
        }

        let positions_data = &positions_layer.unwrap().mesh_data;
        let normals_data = normals_layer.map(|l| &l.mesh_data);
        let uvs_data = uvs_layer.map(|l| &l.mesh_data);

        let mut vertex_offset: u32 = 0;

        for (i, submesh) in mesh.sub_meshes.iter().enumerate() {
            let material_index = submesh.material_index as usize;

            let obj_filename = format!("{}_submesh_{}.obj", output_prefix, i);
            let file = File::create(&obj_filename)?;
            let mut writer = BufWriter::new(file);

            writeln!(writer, "o Submesh_{}", i)?;

            if material_index != 0 {
                // println!("material_index : {}", material_index);
                // println!("texture_name length : {}", texture_name.len());
                // println!("Texture : {:?}", texture_name);

                let material_name = texture_name.get(material_index).unwrap();
                // Always write an MTL reference, even for submesh 0
                let clean_prefix = output_prefix
                    .strip_prefix("output/")
                    .unwrap_or(output_prefix);
                let mtl_filename = format!("{}_submesh_{}.mtl", clean_prefix, i);

                writeln!(writer, "mtllib {}", mtl_filename)?;
                let mtl_filename_path = format!("{}_submesh_{}.mtl", output_prefix, i);

                let mtl_file = File::create(&mtl_filename_path)?;
                let mut mtl_writer = BufWriter::new(mtl_file);

                writeln!(mtl_writer, "newmtl {}", material_name)?;
                writeln!(mtl_writer, "Kd 1.0 1.0 1.0")?;
                writeln!(mtl_writer, "map_Kd {}", material_name)?;

                // println!("🎨 Saved material {} to {}", material_name, mtl_filename);
                writeln!(writer, "usemtl {}", material_name)?;
            }

            // Write vertex positions
            for v in 0..submesh.num_verts {
                let actual_index = vertex_offset + v;
                let offset = (actual_index * 12) as usize;

                if offset + 12 > positions_data.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Vertex data out of bounds",
                    ));
                }

                let px = f32::from_le_bytes(positions_data[offset..offset + 4].try_into().unwrap());
                let py =
                    f32::from_le_bytes(positions_data[offset + 4..offset + 8].try_into().unwrap());
                let pz =
                    f32::from_le_bytes(positions_data[offset + 8..offset + 12].try_into().unwrap());

                writeln!(writer, "v {} {} {}", -px, py, pz)?;
            }

            // Write normals
            if let Some(normals) = normals_data {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 12) as usize;

                    if offset + 12 > normals.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Normal data out of bounds",
                        ));
                    }

                    let nx = f32::from_le_bytes(normals[offset..offset + 4].try_into().unwrap());
                    let ny =
                        f32::from_le_bytes(normals[offset + 4..offset + 8].try_into().unwrap());
                    let nz =
                        f32::from_le_bytes(normals[offset + 8..offset + 12].try_into().unwrap());

                    writeln!(writer, "vn {} {} {}", -nx, ny, nz)?;
                }
            }

            // Write texture coordinates
            if let Some(uvs) = uvs_data {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 8) as usize;

                    if offset + 8 > uvs.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "UV data out of bounds",
                        ));
                    }

                    let u = f32::from_le_bytes(uvs[offset..offset + 4].try_into().unwrap());
                    let v = f32::from_le_bytes(uvs[offset + 4..offset + 8].try_into().unwrap());

                    writeln!(writer, "vt {} {}", u, 1.0 - v)?;
                }
            }

            // Write faces
            for i in (0..submesh.num_indices).step_by(3) {
                let idx1 = submesh.indices[i as usize] + 1;
                let idx2 = submesh.indices[i as usize + 1] + 1;
                let idx3 = submesh.indices[i as usize + 2] + 1;

                if normals_data.is_some() && uvs_data.is_some() {
                    writeln!(
                        writer,
                        "f {}/{}/{} {}/{}/{} {}/{}/{}",
                        idx3, idx3, idx3, idx2, idx2, idx2, idx1, idx1, idx1
                    )?;
                } else if normals_data.is_some() {
                    writeln!(
                        writer,
                        "f {}//{} {}//{} {}//{}",
                        idx3, idx3, idx2, idx2, idx1, idx1
                    )?;
                } else {
                    writeln!(writer, "f {} {} {}", idx3, idx2, idx1)?;
                }
            }

            // println!("✅ Saved submesh {} to {}", i, obj_filename);

            vertex_offset += submesh.num_verts;
        }

        Ok(())
    }

    fn export_to_obj2(&self, mesh: &XACMesh2, output_prefix: &str) -> io::Result<()> {
        let texture_name = self.get_texture_names();

        let positions_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribPositions as u32);

        let normals_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribNormals as u32);

        let uvs_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribUvcoords as u32);

        if positions_layer.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No vertex positions found",
            ));
        }

        let positions_data = &positions_layer.unwrap().mesh_data;
        let normals_data = normals_layer.map(|l| &l.mesh_data);
        let uvs_data = uvs_layer.map(|l| &l.mesh_data);

        let mut vertex_offset: u32 = 0;

        for (i, submesh) in mesh.sub_meshes.iter().enumerate() {
            let material_index = submesh.material_index as usize;

            let obj_filename = format!("{}_submesh_{}.obj", output_prefix, i);
            let file = File::create(&obj_filename)?;
            let mut writer = BufWriter::new(file);

            writeln!(writer, "o Submesh_{}", i)?;

            if material_index != 0 {
                // println!("material_index : {}", material_index);
                // println!("texture_name length : {}", texture_name.len());
                // println!("Texture : {:?}", texture_name);

                let material_name = texture_name.get(material_index).unwrap();
                // Always write an MTL reference, even for submesh 0
                let clean_prefix = output_prefix
                    .strip_prefix("output/")
                    .unwrap_or(output_prefix);
                let mtl_filename = format!("{}_submesh_{}.mtl", clean_prefix, i);

                writeln!(writer, "mtllib {}", mtl_filename)?;
                let mtl_filename_path = format!("{}_submesh_{}.mtl", output_prefix, i);

                let mtl_file = File::create(&mtl_filename_path)?;
                let mut mtl_writer = BufWriter::new(mtl_file);

                writeln!(mtl_writer, "newmtl {}", material_name)?;
                writeln!(mtl_writer, "Kd 1.0 1.0 1.0")?;
                writeln!(mtl_writer, "map_Kd {}", material_name)?;

                // println!("🎨 Saved material {} to {}", material_name, mtl_filename);
                writeln!(writer, "usemtl {}", material_name)?;
            }

            // Write vertex positions
            for v in 0..submesh.num_verts {
                let actual_index = vertex_offset + v;
                let offset = (actual_index * 12) as usize;

                if offset + 12 > positions_data.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Vertex data out of bounds",
                    ));
                }

                let px = f32::from_le_bytes(positions_data[offset..offset + 4].try_into().unwrap());
                let py =
                    f32::from_le_bytes(positions_data[offset + 4..offset + 8].try_into().unwrap());
                let pz =
                    f32::from_le_bytes(positions_data[offset + 8..offset + 12].try_into().unwrap());

                writeln!(writer, "v {} {} {}", -px, py, pz)?;
            }

            // Write normals
            if let Some(normals) = normals_data {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 12) as usize;

                    if offset + 12 > normals.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Normal data out of bounds",
                        ));
                    }

                    let nx = f32::from_le_bytes(normals[offset..offset + 4].try_into().unwrap());
                    let ny =
                        f32::from_le_bytes(normals[offset + 4..offset + 8].try_into().unwrap());
                    let nz =
                        f32::from_le_bytes(normals[offset + 8..offset + 12].try_into().unwrap());

                    writeln!(writer, "vn {} {} {}", -nx, ny, nz)?;
                }
            }

            // Write texture coordinates
            if let Some(uvs) = uvs_data {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 8) as usize;

                    if offset + 8 > uvs.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "UV data out of bounds",
                        ));
                    }

                    let u = f32::from_le_bytes(uvs[offset..offset + 4].try_into().unwrap());
                    let v = f32::from_le_bytes(uvs[offset + 4..offset + 8].try_into().unwrap());

                    writeln!(writer, "vt {} {}", u, 1.0 - v)?;
                }
            }

            // Write faces
            for i in (0..submesh.num_indices).step_by(3) {
                let idx1 = submesh.indices[i as usize] + 1;
                let idx2 = submesh.indices[i as usize + 1] + 1;
                let idx3 = submesh.indices[i as usize + 2] + 1;

                if normals_data.is_some() && uvs_data.is_some() {
                    writeln!(
                        writer,
                        "f {}/{}/{} {}/{}/{} {}/{}/{}",
                        idx3, idx3, idx3, idx2, idx2, idx2, idx1, idx1, idx1
                    )?;
                } else if normals_data.is_some() {
                    writeln!(
                        writer,
                        "f {}//{} {}//{} {}//{}",
                        idx3, idx3, idx2, idx2, idx1, idx1
                    )?;
                } else {
                    writeln!(writer, "f {} {} {}", idx3, idx2, idx1)?;
                }
            }

            // println!("✅ Saved submesh {} to {}", i, obj_filename);

            vertex_offset += submesh.num_verts;
        }

        Ok(())
    }

    fn export_to_struct(&self, mesh: &XACMesh) -> io::Result<Mesh> {
        let texture_name = self.get_texture_names();

        // Find layers by their layer_type_id
        let positions_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribPositions as u32);

        let normals_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribNormals as u32);

        let tangents_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribTangents as u32);

        let uvs_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribUvcoords as u32);

        let colors32_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribColors32 as u32);

        let original_vertex_numbers_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribOrgvtxnumbers as u32);

        let colors128_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribColors128 as u32);

        let bitangents_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribBitangents as u32);

        let positions_data = if let Some(l) = positions_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let normals_data = if let Some(l) = normals_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let tangents_data = if let Some(l) = tangents_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let uvs_data = if let Some(l) = uvs_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let colors32_data = if let Some(l) = colors32_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let original_vertex_numbers_data = if let Some(l) = original_vertex_numbers_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let colors128_data = if let Some(l) = colors128_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let bitangents_data = if let Some(l) = bitangents_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let mut vertex_offset: u32 = 0;
        let mut submeshes = Vec::new();

        for (i, submesh) in mesh.sub_meshes.iter().enumerate() {
            let material_index = submesh.material_index as usize;

            let mut submesh_data = SubMesh {
                texture_name: String::new(),
                position_count: 0,
                positions: Vec::new(),
                normal_count: 0,
                normals: Vec::new(),
                tangent_count: 0,
                tangents: Vec::new(),
                uvcoord_count: 0,
                uvcoords: Vec::new(),
                color32_count: 0,
                colors32: Vec::new(),
                original_vertex_numbers_count: 0,
                original_vertex_numbers: Vec::new(),
                color128_count: 0,
                colors128: Vec::new(),
                bitangent_count: 0,
                bitangents: Vec::new(),
            };

            // Process texture name if material_index is valid
            if material_index != 0 {
                if let Some(material_name) = texture_name.get(material_index) {
                    submesh_data.texture_name = material_name.to_string();
                }
            }

            // Write vertex positions if data exists
            if let Some(positions_layer) = positions_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 12) as usize;

                    if offset + 12 > positions_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Vertex data out of bounds",
                        ));
                    }

                    let px = f32::from_le_bytes(
                        positions_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let py = f32::from_le_bytes(
                        positions_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );
                    let pz = f32::from_le_bytes(
                        positions_data.unwrap()[offset + 8..offset + 12]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.positions.push([-px, py, pz]);
                }
                submesh_data.position_count = submesh_data.positions.len();
            }

            // Write normals if data exists
            if let Some(normals_layer) = normals_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 12) as usize;

                    if offset + 12 > normals_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Normal data out of bounds",
                        ));
                    }

                    let nx = f32::from_le_bytes(
                        normals_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let ny = f32::from_le_bytes(
                        normals_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );
                    let nz = f32::from_le_bytes(
                        normals_data.unwrap()[offset + 8..offset + 12]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.normals.push([-nx, ny, nz]);
                }
                submesh_data.normal_count = submesh_data.normals.len();
            }

            // Write tangents if data exists
            if let Some(tangents_layer) = tangents_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 16) as usize; // 16 bytes for tangent (4 components)

                    if offset + 16 > tangents_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Tangent data out of bounds",
                        ));
                    }

                    let tx = f32::from_le_bytes(
                        tangents_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let ty = f32::from_le_bytes(
                        tangents_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );
                    let tz = f32::from_le_bytes(
                        tangents_data.unwrap()[offset + 8..offset + 12]
                            .try_into()
                            .unwrap(),
                    );
                    let tw = f32::from_le_bytes(
                        tangents_data.unwrap()[offset + 12..offset + 16]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.tangents.push([tx, ty, tz, tw]);
                }
                submesh_data.tangent_count = submesh_data.tangents.len();
            }

            // Write UVs if data exists
            if let Some(uvs_layer) = uvs_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 8) as usize; // 8 bytes for UV (2 components)

                    if offset + 8 > uvs_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "UV data out of bounds",
                        ));
                    }

                    let u = f32::from_le_bytes(
                        uvs_data.unwrap()[offset..offset + 4].try_into().unwrap(),
                    );
                    let v = f32::from_le_bytes(
                        uvs_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.uvcoords.push([u, v]);
                }
                submesh_data.uvcoord_count = submesh_data.uvcoords.len();
            }

            // Write Colors32 if data exists
            if let Some(colors32_layer) = colors32_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 4) as usize; // 4 bytes for color32

                    if offset + 4 > colors32_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Color32 data out of bounds",
                        ));
                    }

                    let r = u32::from_le_bytes(
                        colors32_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.colors32.push(r);
                }
                submesh_data.color32_count = submesh_data.colors32.len();
            }

            // Write Original Vertex Numbers if data exists
            if let Some(original_vertex_numbers_layer) = original_vertex_numbers_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 4) as usize; // 4 bytes for vertex number

                    if offset + 4 > original_vertex_numbers_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Original vertex numbers data out of bounds",
                        ));
                    }

                    let vertex_number = u32::from_le_bytes(
                        original_vertex_numbers_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.original_vertex_numbers.push(vertex_number);
                }
                submesh_data.original_vertex_numbers_count =
                    submesh_data.original_vertex_numbers.len();
            }

            // Write Color128 if data exists
            if let Some(colors128_layer) = colors128_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 16) as usize; // 16 bytes for Color128 (4 components)

                    if offset + 16 > colors128_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Color128 data out of bounds",
                        ));
                    }

                    let r = f32::from_le_bytes(
                        colors128_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let g = f32::from_le_bytes(
                        colors128_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );
                    let b = f32::from_le_bytes(
                        colors128_data.unwrap()[offset + 8..offset + 12]
                            .try_into()
                            .unwrap(),
                    );
                    let a = f32::from_le_bytes(
                        colors128_data.unwrap()[offset + 12..offset + 16]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.colors128.push([r, g, b, a]);
                }
                submesh_data.color128_count = submesh_data.colors128.len();
            }

            // Write Bitangents if data exists
            if let Some(bitangents_layer) = bitangents_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 12) as usize; // 12 bytes for bitangent (3 components)

                    if offset + 12 > bitangents_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Bitangent data out of bounds",
                        ));
                    }

                    let bx = f32::from_le_bytes(
                        bitangents_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let by = f32::from_le_bytes(
                        bitangents_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );
                    let bz = f32::from_le_bytes(
                        bitangents_data.unwrap()[offset + 8..offset + 12]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.bitangents.push([bx, by, bz]);
                }
                submesh_data.bitangent_count = submesh_data.bitangents.len();
            }

            // Add submesh to the list if it has valid data
            if !submesh_data.positions.is_empty()
                || !submesh_data.normals.is_empty()
                || !submesh_data.tangents.is_empty()
                || !submesh_data.uvcoords.is_empty()
                || !submesh_data.colors32.is_empty()
                || !submesh_data.original_vertex_numbers.is_empty()
                || !submesh_data.colors128.is_empty()
                || !submesh_data.bitangents.is_empty()
            {
                submeshes.push(submesh_data);
            }

            vertex_offset += submesh.num_verts;
        }

        // Return the Mesh struct with the submeshes and their count
        Ok(Mesh {
            submesh_count: submeshes.len(),
            submeshes,
        })
    }

    fn export_to_struct2(&self, mesh: &XACMesh2) -> io::Result<Mesh> {
        let texture_name = self.get_texture_names();

        // Find layers by their layer_type_id
        let positions_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribPositions as u32);

        let normals_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribNormals as u32);

        let tangents_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribTangents as u32);

        let uvs_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribUvcoords as u32);

        let colors32_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribColors32 as u32);

        let original_vertex_numbers_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribOrgvtxnumbers as u32);

        let colors128_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribColors128 as u32);

        let bitangents_layer = mesh
            .vertex_attribute_layer
            .iter()
            .find(|layer| layer.layer_type_id == XacAttribute::AttribBitangents as u32);

        let positions_data = if let Some(l) = positions_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let normals_data = if let Some(l) = normals_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let tangents_data = if let Some(l) = tangents_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let uvs_data = if let Some(l) = uvs_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let colors32_data = if let Some(l) = colors32_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let original_vertex_numbers_data = if let Some(l) = original_vertex_numbers_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let colors128_data = if let Some(l) = colors128_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let bitangents_data = if let Some(l) = bitangents_layer {
            Some(&l.mesh_data)
        } else {
            None
        };

        let mut vertex_offset: u32 = 0;
        let mut submeshes = Vec::new();

        for (i, submesh) in mesh.sub_meshes.iter().enumerate() {
            let material_index = submesh.material_index as usize;

            let mut submesh_data = SubMesh {
                texture_name: String::new(),
                position_count: 0,
                positions: Vec::new(),
                normal_count: 0,
                normals: Vec::new(),
                tangent_count: 0,
                tangents: Vec::new(),
                uvcoord_count: 0,
                uvcoords: Vec::new(),
                color32_count: 0,
                colors32: Vec::new(),
                original_vertex_numbers_count: 0,
                original_vertex_numbers: Vec::new(),
                color128_count: 0,
                colors128: Vec::new(),
                bitangent_count: 0,
                bitangents: Vec::new(),
            };

            // Process texture name if material_index is valid
            if material_index != 0 {
                if let Some(material_name) = texture_name.get(material_index) {
                    submesh_data.texture_name = material_name.to_string();
                }
            }

            // Write vertex positions if data exists
            if let Some(positions_layer) = positions_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 12) as usize;

                    if offset + 12 > positions_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Vertex data out of bounds",
                        ));
                    }

                    let px = f32::from_le_bytes(
                        positions_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let py = f32::from_le_bytes(
                        positions_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );
                    let pz = f32::from_le_bytes(
                        positions_data.unwrap()[offset + 8..offset + 12]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.positions.push([-px, py, pz]);
                }
                submesh_data.position_count = submesh_data.positions.len();
            }

            // Write normals if data exists
            if let Some(normals_layer) = normals_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 12) as usize;

                    if offset + 12 > normals_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Normal data out of bounds",
                        ));
                    }

                    let nx = f32::from_le_bytes(
                        normals_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let ny = f32::from_le_bytes(
                        normals_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );
                    let nz = f32::from_le_bytes(
                        normals_data.unwrap()[offset + 8..offset + 12]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.normals.push([-nx, ny, nz]);
                }
                submesh_data.normal_count = submesh_data.normals.len();
            }

            // Write tangents if data exists
            if let Some(tangents_layer) = tangents_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 16) as usize; // 16 bytes for tangent (4 components)

                    if offset + 16 > tangents_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Tangent data out of bounds",
                        ));
                    }

                    let tx = f32::from_le_bytes(
                        tangents_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let ty = f32::from_le_bytes(
                        tangents_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );
                    let tz = f32::from_le_bytes(
                        tangents_data.unwrap()[offset + 8..offset + 12]
                            .try_into()
                            .unwrap(),
                    );
                    let tw = f32::from_le_bytes(
                        tangents_data.unwrap()[offset + 12..offset + 16]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.tangents.push([tx, ty, tz, tw]);
                }
                submesh_data.tangent_count = submesh_data.tangents.len();
            }

            // Write UVs if data exists
            if let Some(uvs_layer) = uvs_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 8) as usize; // 8 bytes for UV (2 components)

                    if offset + 8 > uvs_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "UV data out of bounds",
                        ));
                    }

                    let u = f32::from_le_bytes(
                        uvs_data.unwrap()[offset..offset + 4].try_into().unwrap(),
                    );
                    let v = f32::from_le_bytes(
                        uvs_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.uvcoords.push([u, v]);
                }
                submesh_data.uvcoord_count = submesh_data.uvcoords.len();
            }

            // Write Colors32 if data exists
            if let Some(colors32_layer) = colors32_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 4) as usize; // 4 bytes for color32

                    if offset + 4 > colors32_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Color32 data out of bounds",
                        ));
                    }

                    let r = u32::from_le_bytes(
                        colors32_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.colors32.push(r);
                }
                submesh_data.color32_count = submesh_data.colors32.len();
            }

            // Write Original Vertex Numbers if data exists
            if let Some(original_vertex_numbers_layer) = original_vertex_numbers_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 4) as usize; // 4 bytes for vertex number

                    if offset + 4 > original_vertex_numbers_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Original vertex numbers data out of bounds",
                        ));
                    }

                    let vertex_number = u32::from_le_bytes(
                        original_vertex_numbers_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.original_vertex_numbers.push(vertex_number);
                }
                submesh_data.original_vertex_numbers_count =
                    submesh_data.original_vertex_numbers.len();
            }

            // Write Color128 if data exists
            if let Some(colors128_layer) = colors128_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 16) as usize; // 16 bytes for Color128 (4 components)

                    if offset + 16 > colors128_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Color128 data out of bounds",
                        ));
                    }

                    let r = f32::from_le_bytes(
                        colors128_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let g = f32::from_le_bytes(
                        colors128_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );
                    let b = f32::from_le_bytes(
                        colors128_data.unwrap()[offset + 8..offset + 12]
                            .try_into()
                            .unwrap(),
                    );
                    let a = f32::from_le_bytes(
                        colors128_data.unwrap()[offset + 12..offset + 16]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.colors128.push([r, g, b, a]);
                }
                submesh_data.color128_count = submesh_data.colors128.len();
            }

            // Write Bitangents if data exists
            if let Some(bitangents_layer) = bitangents_layer {
                for v in 0..submesh.num_verts {
                    let actual_index = vertex_offset + v;
                    let offset = (actual_index * 12) as usize; // 12 bytes for bitangent (3 components)

                    if offset + 12 > bitangents_data.unwrap().len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Bitangent data out of bounds",
                        ));
                    }

                    let bx = f32::from_le_bytes(
                        bitangents_data.unwrap()[offset..offset + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let by = f32::from_le_bytes(
                        bitangents_data.unwrap()[offset + 4..offset + 8]
                            .try_into()
                            .unwrap(),
                    );
                    let bz = f32::from_le_bytes(
                        bitangents_data.unwrap()[offset + 8..offset + 12]
                            .try_into()
                            .unwrap(),
                    );

                    submesh_data.bitangents.push([bx, by, bz]);
                }
                submesh_data.bitangent_count = submesh_data.bitangents.len();
            }

            // Add submesh to the list if it has valid data
            if !submesh_data.positions.is_empty()
                || !submesh_data.normals.is_empty()
                || !submesh_data.tangents.is_empty()
                || !submesh_data.uvcoords.is_empty()
                || !submesh_data.colors32.is_empty()
                || !submesh_data.original_vertex_numbers.is_empty()
                || !submesh_data.colors128.is_empty()
                || !submesh_data.bitangents.is_empty()
            {
                submeshes.push(submesh_data);
            }

            vertex_offset += submesh.num_verts;
        }

        // Return the Mesh struct with the submeshes and their count
        Ok(Mesh {
            submesh_count: submeshes.len(),
            submeshes,
        })
    }
}
