# Black-background JPG -> transparent, tightly cropped, centered 1024 PNG.
# Glow-on-black art: alpha = brightness, color un-premultiplied, so the glow fades into alpha.
import sys
import numpy as np
from PIL import Image
from scipy.ndimage import binary_closing, binary_fill_holes

src, dst = sys.argv[1], sys.argv[2]
rgb = np.asarray(Image.open(src).convert("RGB")).astype(np.float32) / 255
lum = rgb.max(axis=2)
floor = 0.06  # JPG noise in the "black"
a = np.clip((lum - floor) / (1 - floor), 0, 1)
color = np.where(a[..., None] > 0, np.clip(rgb / np.maximum(lum, 1e-6)[..., None], 0, 1), 0)
# The dark mouse body would key out half-transparent; keep everything inside the outline opaque.
solid = binary_fill_holes(binary_closing(lum > 0.35, iterations=3))
a[solid] = 1
color[solid] = rgb[solid]
img = Image.fromarray((np.dstack([color, a]) * 255).round().astype(np.uint8), "RGBA")

ys, xs = np.nonzero(a > 0.02)
box = (xs.min(), ys.min(), xs.max() + 1, ys.max() + 1)
img = img.crop(box)
side = int(max(img.size) * 1.08)  # small margin so the glow is not clipped
canvas = Image.new("RGBA", (side, side), (0, 0, 0, 0))
canvas.paste(img, ((side - img.width) // 2, (side - img.height) // 2))
canvas.resize((1024, 1024), Image.LANCZOS).save(dst)
print("crop", box, "->", dst)
