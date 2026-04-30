import base64

# Base64 encoded 1x1 transparent PNG
img_b64 = b"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGMAAQAABQABDQottAAAAABJRU5ErkJggg=="
data = base64.b64decode(img_b64)

with open('src-tauri/icons/icon.png', 'wb') as f:
    f.write(data)
