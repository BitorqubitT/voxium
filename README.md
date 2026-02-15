# GOAL

Writing a DICOM viewer for fun


# Architecture idea
    winit (window + events)
    wgpu (render pipeline)
    render CT slice or ray march into texture
    egui shows that texture
    sliders control uniforms (window/level)


# TODO:
- Get a simple window and display some text
- Interactive button
- Load a DICOM file
- Visualise 2d file
- Button for getting file from disk