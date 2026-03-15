# GOAL

Writing a DICOM viewer for fun


# Architecture idea
    winit (window + events)
    wgpu (render pipeline)
    render CT slice or ray march into texture
    egui shows that texture
    sliders control uniforms (window/level)

data: all datastructs and manipulation of the data (loading, window/level, etc)
render: rendering code (wgpu, shaders, etc)
dicom: all dicom loading and parsing code (maybe move get metadata here too) (not sure yet about this)
viewer: displaying the viewer

app.rs: main app code, event loop




# TODO:
- finish refactor to new structure
- change structs for 3d volume
- implement ray marching
- wgpu to egui?


# Extra: 
- segmentation 