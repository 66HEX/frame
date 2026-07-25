application = defines["app"]  # noqa: F821
background = defines["background"]  # noqa: F821

format = "UDZO"
filesystem = "HFS+"
files = [(application, "Frame.app")]
symlinks = {"Applications": "/Applications"}
hide_extensions = ["Frame.app"]

# Finder includes its 32 pt title bar in the stored window bounds. Keep the
# content area at 600x380 pt so the Retina background is not cropped.
window_rect = ((200, 120), (600, 412))
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
default_view = "icon-view"
show_icon_preview = False

arrange_by = None
label_pos = "bottom"
text_size = 14
icon_size = 150
icon_locations = {
    "Frame.app": (145, 170),
    "Applications": (455, 170),
}
