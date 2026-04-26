# Script to train centropoids from sample vectors
# THIS SCRIPT ISN'T CALLED BY PriEco
# Is here for reproducability
# Author: Roman Lancos <support@prieco.net>
#  License: AGPL v3.0
# Date Created: 2026-02-19
# Last Modified: 2026-02-19
# Usage: Create sample.bin. First 4 bytes are number of vectors. Then insert the vectors. This script is made for 384D vectors but you can change it here. Then call this script and you will get centroids.bin

# Imports
import struct

import faiss
import numpy as np

# Load sample
with open("sample.bin", "rb") as f:
    num_vecs = struct.unpack("I", f.read(4))[0]
    data = np.fromfile(f, dtype=np.float32).reshape(num_vecs, 384)


# Normalize training data (For cosine)
faiss.normalize_L2(data)

# Train
kmeans = faiss.Kmeans(
    d=384,
    k=131072,
    niter=100,
    verbose=True,
    gpu=True,
)
kmeans.train(data)

# Get centroids
centroids = kmeans.centroids.reshape(131072, 384)

# Normalize centroids
faiss.normalize_L2(centroids)

# Save
with open("centroids.bin", "wb") as f:
    f.write(struct.pack("II", 131072, 384))
    centroids.astype(np.float32).tofile(f)
