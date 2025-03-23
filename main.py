# Import the Rust extension module
import toslib

# Define the paths to the IPF and XAC files
ipf_path = "/home/ridwan/Documents/TreeOfSaviorCN/data/bg_hi.ipf"
xac_filename = "barrack_model.xac"

# Call the extract_xac_data_py function
try:
    meshes = toslib.extract_xac_data_py(ipf_path, xac_filename)

    print(f"Number of meshes: {len(meshes)}")

    # Example of accessing mesh data (assuming Mesh has a 'submesh_count' attribute)
    for mesh in meshes:
        print(f"Submesh count: {mesh.submesh_count()}")

        # Limit the output to 1 submeshes
        submesh_count = min(1, len(mesh.submeshes()))

        for i in range(submesh_count):
            submesh = mesh.submeshes()[i]

            print(f"Submesh {i + 1}:")
            print(f"  Texture Name: {submesh.texture_name()}")

            # Print positions (limit to 10)
            position_count = min(10, submesh.position_count())
            print(f"  Positions (showing {position_count}):")
            for j in range(position_count):
                print(f"    {submesh.positions()[j]}")

            # Print normals (limit to 10)
            normal_count = min(10, submesh.normal_count())
            print(f"  Normals (showing {normal_count}):")
            for j in range(normal_count):
                print(f"    {submesh.normals()[j]}")

            # Print tangents (limit to 10)
            tangent_count = min(10, submesh.tangent_count())
            print(f"  Tangents (showing {tangent_count}):")
            for j in range(tangent_count):
                print(f"    {submesh.tangents()[j]}")

            # Print UV coordinates (limit to 10)
            uvcoord_count = min(10, submesh.uvcoord_count())
            print(f"  UV Coordinates (showing {uvcoord_count}):")
            for j in range(uvcoord_count):
                print(f"    {submesh.uvcoords()[j]}")

            # Print colors32 (limit to 10)
            color32_count = min(10, submesh.color32_count())
            print(f"  Colors32 (showing {color32_count}):")
            for j in range(color32_count):
                print(f"    {submesh.colors32()[j]}")

            # Print original vertex numbers (limit to 10)
            original_vertex_numbers_count = min(
                10, submesh.original_vertex_numbers_count()
            )
            print(
                f"  Original Vertex Numbers (showing {original_vertex_numbers_count}):"
            )
            for j in range(original_vertex_numbers_count):
                print(f"    {submesh.original_vertex_numbers()[j]}")

            # Print colors128 (limit to 10)
            color128_count = min(10, submesh.color128_count())
            print(f"  Colors128 (showing {color128_count}):")
            for j in range(color128_count):
                print(f"    {submesh.colors128()[j]}")

            # Print bitangents (limit to 10)
            bitangent_count = min(10, submesh.bitangent_count())
            print(f"  Bitangents (showing {bitangent_count}):")
            for j in range(bitangent_count):
                print(f"    {submesh.bitangents()[j]}")

except Exception as e:
    print(f"Error: {e}")
