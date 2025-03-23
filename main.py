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
        if mesh.submesh_count() > 0:
            for submeshes in mesh.submeshes():
                print(f"Texture Name : {submeshes.texture_name()}")

except Exception as e:
    print(f"Error: {e}")
