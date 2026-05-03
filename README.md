# GOAL
Writing a DICOM viewer for fun

# Architecture idea
    winit (window + events)
    wgpu (render pipeline)
    render CT slice or ray march into texture
    egui shows that texture
    sliders control uniforms (window/level)

# convert to new setup:
- upload to cpu
- data check (order, normalise etc)
- upload to gpu
- render

data: all datastructs and manipulation of the data (loading, window/level, etc)
render: rendering code (wgpu, shaders, etc)
dicom: all dicom loading and parsing code (maybe move get metadata here too) (not sure yet about this)
viewer: displaying the viewer

app.rs: main app code, event loop

# TODO:
- implement dummy 3d rendering -> cube?
- change structs for 3d volume
- implement ray marching

# Extra: 
- segmentation 

new setup:
Unified Data Flow: All image data through ImageSource, eliminating direct manipulation of VolumeCpu or DynamicImage in the viewer.
Consistent API: The viewer only deals with ImageSource

ImageSource::Single handles individual images
ImageSource::Volume manages 3D volumes

make everything work through imagesource