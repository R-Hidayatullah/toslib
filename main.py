import bpy
import toslib

# Define the paths to the IPF and XAC files
ipf_path = "/home/ridwan/Documents/TreeOfSaviorCN/data/bg_hi.ipf"
xac_filename = "barrack_model.xac"

# Adjust UVs for OpenGL if coming from DirectX (flip Y-axis)
def adjust_uv_for_opengl(uvcoords):
    for uv in uvcoords:
        uv[1] = 1.0 - uv[1]  # Flip the Y component
    return uvcoords

# Function to create a mesh from extracted data
def create_custom_mesh(mesh_data, group_name, collection):
    for i, submesh_data in enumerate(mesh_data.submeshes()):
        # Extract all the data from the submesh
        texture_name = submesh_data.texture_name()  # Texture name
        positions = submesh_data.positions()  # Positions (vertices)
        normals = submesh_data.normals()  # Normals
        tangents = submesh_data.tangents()  # Tangents
        uvcoords = submesh_data.uvcoords()  # UV coordinates
        bitangents = submesh_data.bitangents()  # Bitangents
        indices = submesh_data.indices()  # Indices

        # Flip UV coordinates if necessary for OpenGL
        uvcoords = adjust_uv_for_opengl(uvcoords)

        # Print indices for debugging
        print(f"Submesh {i} Indices: {indices[:30]}")  # Print the first 30 indices for debugging

        # Create a new mesh object in Blender
        mesh = bpy.data.meshes.new(name=f"{group_name}_Mesh_{i}")
        mesh_object = bpy.data.objects.new(f"{group_name}_Mesh_{i}", mesh)

        # Link the object to the collection (organized by xac_filename)
        collection.objects.link(mesh_object)

        # Set the object as the active one
        bpy.context.view_layer.objects.active = mesh_object
        mesh_object.select_set(True)

        # Create faces using indices
        faces = []
        for idx in range(0, len(indices), 3):
            if idx + 2 < len(indices):  # Ensure the face has at least 3 indices
                face = (indices[idx], indices[idx + 1], indices[idx + 2])
                faces.append(face)

        # Create the mesh with positions, faces, and normals
        mesh.from_pydata(positions, [], faces)

        # Set normals for the mesh
        normals_for_loops = []
        for face in faces:
            for vertex in face:
                normals_for_loops.append(normals[vertex])
        mesh.normals_split_custom_set(normals_for_loops)

        # Set the UVs for each face
        uv_layer = mesh.uv_layers.new(name="UVMap")
        for i, face in enumerate(faces):
            for j, vertex_idx in enumerate(face):
                uv_layer.data[i * 3 + j].uv = uvcoords[vertex_idx]

        # Apply tangents and bitangents if available
        if tangents and len(tangents) == len(positions):
            # Create a custom tangent attribute
            mesh.attributes.new(name="Tangent", type='FLOAT_VECTOR', domain='POINT')
            tangent_layer = mesh.attributes["Tangent"]
            for idx, tangent in enumerate(tangents):
                tangent_layer.data[idx].vector = tangent

        if bitangents and len(bitangents) == len(positions):
            # Create a custom bitangent attribute
            mesh.attributes.new(name="Bitangent", type='FLOAT_VECTOR', domain='POINT')
            bitangent_layer = mesh.attributes["Bitangent"]
            for idx, bitangent in enumerate(bitangents):
                bitangent_layer.data[idx].vector = bitangent

        # Update the mesh data
        mesh.update()

        # Optionally, apply the texture to the material
        apply_texture(mesh_object, texture_name, group_name)


# Function to apply texture to the mesh and set material name based on texture name
def apply_texture(mesh_object, texture_name, group_name):
    # Ensure the object has a material, named based on the texture name
    material_name = f"{texture_name}"
    if not mesh_object.data.materials.get(material_name):
        material = bpy.data.materials.new(name=material_name)
        mesh_object.data.materials.append(material)
    else:
        material = mesh_object.data.materials[material_name]

    material.use_nodes = True
    bsdf = material.node_tree.nodes.get("Principled BSDF")

    # Load the texture
    try:
        texture = bpy.data.images.load(texture_name)
    except RuntimeError as e:
        print(f"Error loading texture {texture_name}: {e}")
        return
    
    # Create texture node
    tex_image = material.node_tree.nodes.new(type="ShaderNodeTexImage")
    tex_image.image = texture

    # Connect texture to the BSDF shader
    material.node_tree.links.new(tex_image.outputs["Color"], bsdf.inputs["Base Color"])

# Extract meshes using toslib and group them by xac_filename
try:
    meshes = toslib.extract_xac_data_py(ipf_path, xac_filename)
except Exception as e:
    print(f"Error extracting meshes: {e}")
    meshes = []

# Group meshes by xac_filename (group_name) and create Blender objects
group_name = xac_filename.replace(".xac", "")  # Use the xac_filename as the group name (remove the extension)

# Create a collection for the xac_filename group
if group_name not in bpy.data.collections:
    collection = bpy.data.collections.new(group_name)
    bpy.context.scene.collection.children.link(collection)
else:
    collection = bpy.data.collections[group_name]

# Iterate through each mesh and create Blender objects within the collection
for mesh in meshes:
    create_custom_mesh(mesh, group_name, collection)
