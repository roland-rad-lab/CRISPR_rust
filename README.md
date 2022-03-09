# CRISPR\_rust
Extraction of CRISPR counts written in rust.

## Usage
This program takes two bam files and returns rows based on the target names in the headers (usually your guide names). By default it will return a row for the product of the target names. Alternatively specify --pair to return a row for each pair of target names, taking one from each bam (both files must have the same number and the sequence names must be in the desired order).

### Options
| type      | long           | Default    | Details                                                                                                                                                                                                               |
|-----------|----------------|------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Flag      | --help         |            | Display usage                                                                                                                                                                                                         |
| uint      | --n-mismatch   | 2          | The value of the SAM format field (specified by --tag-mismatch) must be less than this value.                                                                                                                         |
| String    | --output-tsv   | counts.tsv | The name of the file where the output counts will be written as tab separated values.                                                                                                                                 |
| Flag      | --pair         |            | A row will be output for every pair of sequence names from BAM\_R1 and BAM\_R2 (whose headers must have an identical number of sequence names and be in the desired order). The default returns rows for the product. |                          
| String    | --tag-mismatch | NM         | The name of the SAM format field containing the unsigned integer which will be filtered by comparison with --n-mismatch. Alignments where the value is less than --n-mismatch contribute to counts.                   |
| String    | <SAMPLE\_NAME> |            | This value will appear in the Sample\_Name column of counts.long.tsv and as the column name in counts.wide.tsv (See https://github.com/roland-rad-lab/CRISPR\_analysis)                                               |
| BAM File  | <BAM\_R1>      |            | BAM file for read one.                                                                                                                                                                                                |
| BAM File  | <BAM\_R2>      |            | BAM file for read two.                                                                                                                                                                                                |

### Packaging as a container
The included Dockerfile can be used to create a container for this program.
```bash
# Build is an empty directory or contains things we wish to include in our image (sometimes called context)
mkdir -p build
curl -L https://raw.githubusercontent.com/roland-rad-lab/CRISPR_rust/main/Dockerfile.crispr-rust > build/Dockerfile.crispr-rust

# Using Docker
cd build && sudo docker build --force-rm --no-cache -t crispr-rust-0.0.3 -f Dockerfile.crispr-rust .

# Using podman (open source alternative to Docker)
# podman machine init --disk-size 50 --memory 4096
# podman machine start
podman --remote build --no-cache --rm --tag crispr-rust-0.0.3 -f build/Dockerfile.crispr-rust build

```

From here we can optionally choose to publish our image to an image repository so others can download it. In this case to our public repo at [LRZ gitlab](https://gitlab.lrz.de/roland-rad-lab/images-public/container_registry).

```bash

## Using Docker

# Our image name must exactly match our remote destination
docker tag crispr-rust-0.0.3 gitlab.lrz.de:5005/roland-rad-lab/images-public/crispr-rust:0.0.3
# Now we need to login using the access token with write permissions
docker login gitlab.lrz.de:5005 -u SECRET_GITLAB_ACCESS_TOKEN_NAME -p SECRET_GITLAB_ACCESS_TOKEN
docker push gitlab.lrz.de:5005/roland-rad-lab/images-public/crispr-rust:0.0.3

## Using podman (as an alternative to Docker)

# Our image name must exactly match our remote destination (pushing an image with an already existing tag probably won't work)
podman tag localhost/crispr-rust-0.0.3:latest gitlab.lrz.de:5005/roland-rad-lab/images-public/crispr-rust:0.0.3
# Now we need to login using the access token with write permissions
podman login gitlab.lrz.de:5005 -u SECRET_GITLAB_ACCESS_TOKEN_NAME -p SECRET_GITLAB_ACCESS_TOKEN
podman push gitlab.lrz.de:5005/roland-rad-lab/images-public/crispr-rust:0.0.3


```

Or we can export our image as a gzipped tarball and subsequently copy to and extract it on the system where we would like to use it.

```bash
# Optionally fetch image from a container repository (if you didn't build it locally)

## Using Docker
# docker login gitlab.lrz.de:5005 -u roland-rad-lab-images-public -p xUSAxe6kmRb1y13Ajzak
# docker pull gitlab.lrz.de:5005/roland-rad-lab/images-public/crispr-rust:0.0.3
# ch-builder2tar is part of https://github.com/hpc/charliecloud
ch-builder2tar -b docker gitlab.lrz.de:5005/roland-rad-lab/images-public/crispr-rust:0.0.3 target/


## Using podman
# podman login gitlab.lrz.de:5005 -u roland-rad-lab-images-public -p xUSAxe6kmRb1y13Ajzak
# podman pull gitlab.lrz.de:5005/roland-rad-lab/images-public/crispr-rust:0.0.3
# I opened an issue to add podman support to ch-builder2tar, including my implementation
# https://github.com/hpc/charliecloud/issues/1256


```


