# Image and Figure Extraction in PDF

## 1. Image XObjects

PDF images are most commonly embedded as **Image XObjects**. An XObject is an indirect object whose dictionary contains `/Type /XObject` and `/Subtype /Image`. It is invoked from a content stream using the `Do` operator:

```
/ImageName Do
```

where `ImageName` is a name key in the current resource dictionary's `/XObject` subdictionary that maps to the image's indirect reference.

The XObject dictionary must contain:

| Key | Type | Description |
|---|---|---|
| `Width` | integer | Pixel width of the raster |
| `Height` | integer | Pixel height of the raster |
| `ColorSpace` | name or array | Color space of the image samples |
| `BitsPerComponent` | integer | Bits per color component (1, 2, 4, 8, or 16) |
| `Filter` | name or array | Compression filter(s) applied to the stream |

The stream body following the dictionary contains the raw (filtered) image data. BitsPerComponent is omitted when the image uses a mask or JBIG2 encoding where bit depth is implied.

### Positioning via the CTM

The `Do` operator renders the image into a 1×1 unit square anchored at the origin. The **current transformation matrix (CTM)** at the point of `Do` invocation maps that unit square into page space. A canonical image placement looks like:

```
q
72 0 0 96 144 432 cm
/Im1 Do
Q
```

The `cm` operator concatenates `[72 0 0 96 144 432]` onto the CTM: this scales the unit square to 72×96 points and translates its origin to (144, 432) in page coordinates. The rendered bounding box in page units is thus derived from the CTM columns — specifically, the x-extent is the length of the first column vector and the y-extent is the length of the second column vector. When the matrix contains rotation or shear, the bounding box must be computed as the convex hull of the four transformed corners: `(0,0)`, `(1,0)`, `(0,1)`, `(1,1)`.

## 2. Inline Images

Inline images embed pixel data directly into the content stream using a three-operator sequence:

```
BI
  /W 320 /H 240 /CS /RGB /BPC 8 /F /DCT
ID
<binary image data>
EI
```

The `BI` (Begin Image) operator introduces the inline dictionary. Key abbreviations are standardized:

| Abbreviation | Full key |
|---|---|
| `/W` | `Width` |
| `/H` | `Height` |
| `/CS` | `ColorSpace` |
| `/BPC` | `BitsPerComponent` |
| `/F` | `Filter` |
| `/DP` | `DecodeParms` |

`ID` (Image Data) marks the transition from dictionary to binary payload. The parser must switch to raw byte mode immediately after the whitespace following `ID`. The payload ends at the next unescaped `EI` token. Reliably detecting `EI` requires either tracking the filter's expected byte count or scanning for `EI` preceded by a whitespace character.

Inline images are limited to simpler use cases: they cannot be referenced by name, cannot be reused across content streams, and are typically restricted to JPEG, CCITT, or uncompressed data. They carry no indirect object overhead but complicate stream parsing significantly.

## 3. Filter Decoding

Both XObject streams and inline images may be compressed with one or more filters listed in the `/Filter` key (a name for a single filter, or an array for a chain). Filters are applied in array order during encoding; decoding reverses the chain.

**Common filters:**

- **`DCTDecode`** — JPEG (ISO 10918). The stream is a complete JFIF/JPEG file. `DecodeParms` may specify `ColorTransform` (0 = no transform, 1 = YCbCr→RGB, -1 = automatic). Standard JPEG decoders handle the DCT coefficients, quantization, and Huffman decoding.
- **`JPXDecode`** — JPEG 2000 (ISO 15444). The stream is a complete JP2 or J2C codestream. Color space information may be embedded in the JP2 container; when present it overrides the PDF `/ColorSpace` key.
- **`JBIG2Decode`** — Bi-level (1 bpp) compression. `DecodeParms` may contain a `JBIG2Globals` key whose value is a stream containing the global segment data that must be prepended before decoding. Requires a full JBIG2 decoder (e.g., the `jbig2dec` library via FFI).
- **`CCITTFaxDecode`** — Group 3 or Group 4 fax encoding. `DecodeParms` specifies `K` (0=Group3 1D, -1=Group4, positive=Group3 2D with K rows between EOL), `Columns`, `Rows`, `BlackIs1`, `EncodedByteAlign`.
- **`FlateDecode`** — zlib/deflate (RFC 1950 wrapper). `DecodeParms` may specify a PNG predictor via `Predictor` (10–15 for PNG filter types None/Sub/Up/Average/Paeth applied row-by-row). After inflation, the predictor must be undone row by row.
- **`LZWDecode`** — LZW compression. `DecodeParms` supports `EarlyChange` (1 by default, meaning the code size increases one code early). The LZW variant matches the TIFF LZW convention, not GIF.
- **`RunLengthDecode`** — PackBits run-length encoding. Each control byte `n` signals either `(257-n)` copies of the next byte (if n > 128) or `(n+1)` literal bytes (if n < 128). Byte 128 signals end-of-data.

For chained filters — e.g., `[/ASCII85Decode /FlateDecode]` — the decoder applies ASCII85 first to produce binary, then inflates the result.

## 4. Color Spaces

The `/ColorSpace` value determines how to interpret decoded sample bytes.

**Device spaces** are the simplest: `DeviceGray` (1 component, 0=black, 1=white), `DeviceRGB` (3 components), `DeviceCMYK` (4 components, subtractive).

**Calibrated spaces** embed a viewing condition: `CalGray` and `CalRGB` specify a `WhitePoint`, optional `BlackPoint`, and `Gamma`/`Matrix`. `Lab` uses the CIE L*a*b* model with `WhitePoint` and `Range` bounds on a* and b*.

**`ICCBased`** references an embedded ICC profile stream. The profile's `N` value gives the component count. ICC profiles provide a precise device-independent color path; for sRGB output, apply the ICC forward transform to XYZ and then the sRGB matrix.

**`Indexed`** defines a palette: `[/Indexed base hival lookup]`. The base space specifies the color model of palette entries; `hival` is the maximum index (palette has `hival+1` entries); `lookup` is either a string or stream of `(hival+1) * N` bytes where N is the component count of the base space. Each 8-bit sample is a palette index.

**`Separation`** addresses a single named colorant: `[/Separation name alternateSpace tintTransform]`. The tint transform (a PDF function) maps a tint value [0,1] to the alternate space. When the target device does not support the colorant, apply the tint transform as a fallback to the alternate space (which is typically `DeviceCMYK` or `DeviceRGB`).

**`DeviceN`** generalizes Separation to multiple colorants: `[/DeviceN names alternateSpace tintTransform attributes]`. Each channel maps to a named colorant; the tint transform maps the N-component input to the alternate space.

For pipeline output, convert all color spaces to sRGB: device spaces use standard matrices (CMYK to RGB: `R=1-min(1,C+K)`, `G=1-min(1,M+K)`, `B=1-min(1,Y+K)`); calibrated/ICC spaces go through XYZ intermediate.

## 5. Image Geometry

The CTM at `Do` invocation is a 3×3 affine matrix stored as six values `[a b c d e f]`, representing:

```
| a  b  0 |
| c  d  0 |
| e  f  1 |
```

The rendered width in page units is `sqrt(a² + b²)` and the rendered height is `sqrt(c² + d²)`. Rotation angle is `atan2(b, a)`. Shear is present when the dot product `(a·c + b·d)` is nonzero.

DPI is computed as:

```
dpi_x = Width_px / (rendered_width_pts / 72.0)
dpi_y = Height_px / (rendered_height_pts / 72.0)
```

For rotated images, the bounding box in axis-aligned page coordinates is the AABB of the four corners produced by transforming `(0,0)`, `(1,0)`, `(0,1)`, `(1,1)` through the CTM.

## 6. Form XObjects

A `/Subtype /Form` XObject is a self-contained reusable content stream — not an AcroForm widget. It may embed text, images, paths, and other XObjects (including nested Form XObjects). The parser must recurse into each Form XObject encountered via `Do`.

The Form XObject dictionary includes:

- `/BBox` — the bounding box of the form in its own local coordinate system.
- `/Matrix` (optional) — a transformation applied before the form's content stream executes, in addition to the invoking CTM.
- `/Resources` — its own resource dictionary, independent of the page's.

When a `Do` operator names a Form XObject, the renderer pushes the current graphics state, concatenates the form's `/Matrix` onto the CTM, clips to `/BBox`, then executes the content stream. The combined CTM (page CTM × form matrix) must be tracked at every `Do` invocation inside the form to correctly compute image geometry.

## 7. Soft Masks and Transparency

Images participate in the PDF transparency model in three ways:

- **`ImageMask`** (boolean) — when `true`, the image is a 1-bpp stencil. Samples with value 0 paint the current color; samples with value 1 are transparent. `ColorSpace` and `BitsPerComponent` are not used.
- **`/Mask`** — either a color key mask (an array of `[min max]` pairs per component defining transparent ranges) or a reference to a 1-bpp image stream serving as a hard mask.
- **`/SMask`** — a reference to a grayscale image stream interpreted as an alpha channel (0=fully transparent, 255=fully opaque). The SMask stream is itself an Image XObject with its own filter, width, height, and `ColorSpace` of `DeviceGray`.

For figure detection purposes, an image that is pure stencil (`ImageMask=true`) or has very low average alpha may be a decorative overlay rather than a content figure.

## 8. Detecting Figure Regions

Building the figure inventory for a page:

1. Walk the content stream, tracking the CTM stack. At each `Do` invocation for an Image XObject, compute the AABB bounding box in page coordinates and record the image metadata.
2. Apply size thresholds: images smaller than a minimum area threshold (e.g., 1% of page area) are likely icons or decorative glyphs; images covering more than 90% of the page are likely scanned-page backgrounds.
3. Apply position heuristics: watermarks are typically centered and semi-transparent; logos appear near page margins and are small; content figures appear in the body region with substantial rendered area.
4. Caption association: scan for text runs within a vertical proximity band (e.g., ±2× line-height) below or above the image bounding box. Text beginning with "Figure", "Fig.", or a numeric pattern is a strong caption signal. Associate the nearest qualifying text run as the figure's caption.
5. Detect full-page images: rendered size within 5% of the page's MediaBox and positioned at or near the origin — flag as `scanned_page`.

## 9. Output Representation

Each detected figure is emitted as a JSON object in the extraction output:

```json
{
  "kind": "figure",
  "page": 3,
  "bbox": [72.0, 300.0, 540.0, 600.0],
  "width_px": 1200,
  "height_px": 900,
  "dpi": 144.0,
  "color_space": "DeviceRGB",
  "filter": ["DCTDecode"],
  "caption": "Figure 4. Loss curves over 100 training epochs.",
  "image_b64": "<base64-encoded PNG or raw raster, present only if extract_images=true>"
}
```

`bbox` is `[x_min, y_min, x_max, y_max]` in PDF page units (origin at bottom-left per PDF convention, or converted to top-left depending on the caller's coordinate system preference). `dpi` is the effective horizontal DPI rounded to two decimal places. `filter` is the original filter chain as decoded from the XObject dictionary. `caption` is null when no caption is detected.

## 10. Extraction Use Cases and Caller Options

Decoding image bytes is expensive — allocating a decompressed raster for a 300 DPI full-page image at A4 size requires ~25 MB. The extraction pipeline exposes two caller-controlled options:

- **`extract_images: bool`** — when `false`, only metadata (`bbox`, `width_px`, `height_px`, `dpi`, `color_space`, `filter`) is emitted. The image stream is not decompressed. This is the default for text-extraction workflows where image content is not needed.
- **`max_image_dpi: u32`** — when `extract_images` is `true`, images whose effective DPI exceeds this threshold are downsampled before encoding. The downsampled dimensions are `round(rendered_width_pts / 72.0 * max_image_dpi)` × `round(rendered_height_pts / 72.0 * max_image_dpi)`. A common default is 150 DPI for document previews or 300 DPI for archival quality.

For very large images (e.g., 10,000×10,000 px TIFF-equivalent embedded as FlateDecode), the decoder should process row-by-row rather than inflating the entire stream into a contiguous buffer. FlateDecode with PNG predictors naturally supports row-granularity streaming: inflate one PNG-filtered row, unfilter it (applying Sub/Up/Average/Paeth as indicated), emit to the output buffer, then continue. This keeps peak memory bounded to `2 × stride_bytes` regardless of image height.

JBIG2 and JPEG 2000 streams require external codec libraries; callers without FFI dependencies available should fall back to emitting raw stream bytes under a `raw_stream_b64` key rather than failing. The `filter` field in the output indicates which codec is needed for the caller to decode the bytes independently.
