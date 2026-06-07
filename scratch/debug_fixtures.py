#!/usr/bin/env python3
import pikepdf

# Check linearization_toggle fixtures
for ver in ["v1", "v2"]:
    path = f"tests/fingerprint/fixtures/linearization_toggle/{ver}.pdf"
    print(f"\n{path}:")
    try:
        with pikepdf.open(path) as pdf:
            print(f"  Pages: {len(pdf.pages)}")
            root_ref = pdf.trailer.get("/Root")
            print(f"  Trailer/Root: {root_ref}")

            # Check if linearized
            if "/Linearized" in pdf.Root:
                lin = pdf.Root["/Linearized"]
                print(f"  Linearized: {lin}")

            # Get the actual root object
            root = pdf.Root
            print(f"  Root type: {type(root)}")

            # Check if /Pages key exists
            if "/Pages" in root:
                pages_ref = root["/Pages"]
                print(f"  /Pages reference: {pages_ref}")

    except Exception as e:
        print(f"  Error: {e}")

# Check content_edit fixtures
print("\n--- content_edit fixtures ---")
for ver in ["v1", "v2"]:
    path = f"tests/fingerprint/fixtures/content_edit_one_glyph/{ver}.pdf"
    print(f"\n{path}:")
    try:
        with pikepdf.open(path) as pdf:
            page = pdf.pages[0]
            if "/Contents" in page:
                contents = page["/Contents"]
                if hasattr(contents, "read_bytes"):
                    data = contents.read_bytes()
                else:
                    data = bytes(contents)
                print(f"  Content stream: {data[:100]}")
                print(f"  Length: {len(data)}")
    except Exception as e:
        print(f"  Error: {e}")
