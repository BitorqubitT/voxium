# GOAL

Writing a DICOM viewer for fun


# Architecture idea
    winit (window + events)
    wgpu (render pipeline)
    render CT slice or ray march into texture
    egui shows that texture
    sliders control uniforms (window/level)


# TODO:
- cleanup load directory
- cleanup get metadata
- display metadata in ui
