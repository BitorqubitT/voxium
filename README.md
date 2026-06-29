# GOAL
Writing a DICOM viewer for fun

# Architecture idea
    winit (window + events)
    wgpu (render pipeline)
    render CT slice or ray march into texture
    egui shows that texture
    sliders control uniforms (window/level)

# TODO:
- cleanup code
- create rendering pipeline instead of putting everything in render volume
- Want to be able to create multipleviews (one 3d camera one 2d slice inspector)
- Let app change viewtransform for zooming etc.


new setup:
At the moment it works like this:
- load images to cpu
- do some augmentations and check order
- load images to gpu
- Pass image to viewer
- Viewer creates a second canvas where it renders the test "slice"
- Send view to egui via widget and ui
  

# Extra: 
- segmentation 
