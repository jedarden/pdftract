//! Generate AcroForm and XFA PDF test fixtures.
//!
//! This program creates PDF test files with form fields for Phase 7.4 testing:
//! - acroform-text-fields.pdf: AcroForm with text, checkbox, radio, and dropdown fields
//! - acroform-readonly.pdf: AcroForm with pre-filled read-only fields
//! - acroform-submit.pdf: AcroForm with a submit button
//! - xfa-dynamic.pdf: XFA dynamic form (placeholder for future XFA support)
//!
//! Each fixture includes corresponding .json ground truth with expected field values.

use lopdf::dictionary;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::fs::File;
use std::io::Write;

fn create_simple_page(content: &[u8], doc: &mut Document) -> ObjectId {
    let mut page_dict = Dictionary::new();
    page_dict.set("Type", "Page");
    page_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));
    page_dict.set("Resources", dictionary! {
        "Font" => dictionary! {
            "F1" => dictionary! {
                "Type" => "Font",
                "Subtype" => "Type1",
                "BaseFont" => "Helvetica"
            }
        }
    });

    let content_stream_id = doc.new_object_id();
    doc.objects.insert(content_stream_id, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        content.to_vec()
    )));
    page_dict.set("Contents", Object::Reference(content_stream_id));

    doc.add_object(page_dict)
}

fn create_acroform_text_fields_pdf() {
    let mut doc = Document::with_version("1.4");

    // Create page
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(AcroForm Test: Text Fields) Tj\nET\n";
    let page_id = create_simple_page(content, &mut doc);

    // Create pages dict
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    let pages_id = doc.add_object(pages_dict);

    // Update page parent
    if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Create AcroForm fields
    // Field 1: Text field with value
    let mut field1_dict = Dictionary::new();
    field1_dict.set("T", Object::String(b"employee_name".to_vec(), false));
    field1_dict.set("FT", Object::Name(b"Tx".to_vec()));
    field1_dict.set("V", Object::String(b"John Doe".to_vec(), false));
    field1_dict.set("DV", Object::String(b"Jane Doe".to_vec(), false));
    field1_dict.set("Ff", Object::Integer(2)); // Required flag
    field1_dict.set("MaxLen", Object::Integer(50));
    field1_dict.set("Rect", Object::Array(vec![
        Object::Real(100.0), Object::Real(650.0),
        Object::Real(300.0), Object::Real(670.0)
    ]));
    let field1_id = doc.add_object(field1_dict);

    // Field 2: Multiline text field
    let mut field2_dict = Dictionary::new();
    field2_dict.set("T", Object::String(b"address".to_vec(), false));
    field2_dict.set("FT", Object::Name(b"Tx".to_vec()));
    field2_dict.set("V", Object::String(b"123 Main St\nAnytown, USA".to_vec(), false));
    field2_dict.set("Ff", Object::Integer(1 << 12)); // Multiline flag
    field2_dict.set("Rect", Object::Array(vec![
        Object::Real(100.0), Object::Real(600.0),
        Object::Real(400.0), Object::Real(650.0)
    ]));
    let field2_id = doc.add_object(field2_dict);

    // Field 3: Checkbox (checked)
    let mut field3_dict = Dictionary::new();
    field3_dict.set("T", Object::String(b"is_manager".to_vec(), false));
    field3_dict.set("FT", Object::Name(b"Btn".to_vec()));
    field3_dict.set("V", Object::Name(b"Yes".to_vec()));
    field3_dict.set("DV", Object::Name(b"Off".to_vec()));
    field3_dict.set("Rect", Object::Array(vec![
        Object::Real(100.0), Object::Real(570.0),
        Object::Real(115.0), Object::Real(585.0)
    ]));
    let field3_id = doc.add_object(field3_dict);

    // Field 4: Radio button group with two options
    let mut radio_group_dict = Dictionary::new();
    radio_group_dict.set("T", Object::String(b"department".to_vec(), false));
    radio_group_dict.set("FT", Object::Name(b"Btn".to_vec()));
    radio_group_dict.set("Ff", Object::Integer(1 << 24)); // Radio flag
    radio_group_dict.set("V", Object::Name(b"sales".to_vec()));

    let mut radio_option1_dict = Dictionary::new();
    radio_option1_dict.set("T", Object::String(b"sales".to_vec(), false));
    radio_option1_dict.set("FT", Object::Name(b"Btn".to_vec()));
    radio_option1_dict.set("Ff", Object::Integer(1 << 24)); // Radio
    radio_option1_dict.set("V", Object::Name(b"sales".to_vec()));
    radio_option1_dict.set("Rect", Object::Array(vec![
        Object::Real(200.0), Object::Real(540.0),
        Object::Real(215.0), Object::Real(555.0)
    ]));
    let radio_option1_id = doc.add_object(radio_option1_dict);

    let mut radio_option2_dict = Dictionary::new();
    radio_option2_dict.set("T", Object::String(b"engineering".to_vec(), false));
    radio_option2_dict.set("FT", Object::Name(b"Btn".to_vec()));
    radio_option2_dict.set("Ff", Object::Integer(1 << 24)); // Radio
    radio_option2_dict.set("V", Object::Name(b"Off".to_vec()));
    radio_option2_dict.set("Rect", Object::Array(vec![
        Object::Real(200.0), Object::Real(520.0),
        Object::Real(215.0), Object::Real(535.0)
    ]));
    let radio_option2_id = doc.add_object(radio_option2_dict);

    radio_group_dict.set("Kids", Object::Array(vec![
        Object::Reference(radio_option1_id),
        Object::Reference(radio_option2_id),
    ]));
    let radio_group_id = doc.add_object(radio_group_dict);

    // Field 5: Dropdown (Choice field with combo flag)
    let mut field5_dict = Dictionary::new();
    field5_dict.set("T", Object::String(b"role".to_vec(), false));
    field5_dict.set("FT", Object::Name(b"Ch".to_vec()));
    field5_dict.set("V", Object::String(b"developer".to_vec(), false));
    field5_dict.set("Ff", Object::Integer(1 << 17)); // Combo flag
    field5_dict.set("Opt", Object::Array(vec![
        Object::String(b"manager".to_vec(), false),
        Object::String(b"developer".to_vec(), false),
        Object::String(b"designer".to_vec(), false),
    ]));
    field5_dict.set("Rect", Object::Array(vec![
        Object::Real(100.0), Object::Real(490.0),
        Object::Real(300.0), Object::Real(510.0)
    ]));
    let field5_id = doc.add_object(field5_dict);

    // Create AcroForm dict
    let mut acroform_dict = Dictionary::new();
    acroform_dict.set("Fields", Object::Array(vec![
        Object::Reference(field1_id),
        Object::Reference(field2_id),
        Object::Reference(field3_id),
        Object::Reference(radio_group_id),
        Object::Reference(field5_id),
    ]));
    let acroform_id = doc.add_object(acroform_dict);

    // Create catalog
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));
    catalog_dict.set("AcroForm", Object::Reference(acroform_id));

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Save PDF
    doc.save("tests/fixtures/forms/acroform-text-fields.pdf").unwrap();
    println!("Created acroform-text-fields.pdf");

    // Create ground truth JSON
    let ground_truth = r#"{
  "form_fields": [
    {
      "name": "employee_name",
      "field_type": "Tx",
      "value": "John Doe",
      "default_value": "Jane Doe",
      "flags": {
        "read_only": false,
        "required": true,
        "multiline": false,
        "password": false
      },
      "max_length": 50
    },
    {
      "name": "address",
      "field_type": "Tx",
      "value": "123 Main St\nAnytown, USA",
      "default_value": null,
      "flags": {
        "read_only": false,
        "required": false,
        "multiline": true,
        "password": false
      },
      "max_length": null
    },
    {
      "name": "is_manager",
      "field_type": "Btn",
      "value": "Yes",
      "default_value": "Off",
      "flags": {
        "read_only": false,
        "required": false,
        "radio": false,
        "pushbutton": false
      },
      "button_kind": "checkbox",
      "checked": true
    },
    {
      "name": "department.sales",
      "field_type": "Btn",
      "value": "sales",
      "default_value": null,
      "flags": {
        "read_only": false,
        "required": false,
        "radio": true,
        "pushbutton": false
      },
      "button_kind": "radio",
      "checked": true
    },
    {
      "name": "department.engineering",
      "field_type": "Btn",
      "value": "Off",
      "default_value": null,
      "flags": {
        "read_only": false,
        "required": false,
        "radio": true,
        "pushbutton": false
      },
      "button_kind": "radio",
      "checked": false
    },
    {
      "name": "role",
      "field_type": "Ch",
      "value": "developer",
      "default_value": null,
      "flags": {
        "read_only": false,
        "required": false,
        "combo": true,
        "multi_select": false
      },
      "options": ["manager", "developer", "designer"]
    }
  ]
}"#;

    let mut json_file = File::create("tests/fixtures/forms/acroform-text-fields.json").unwrap();
    json_file.write_all(ground_truth.as_bytes()).unwrap();
    println!("Created acroform-text-fields.json");
}

fn create_acroform_readonly_pdf() {
    let mut doc = Document::with_version("1.4");

    // Create page
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(Read-Only AcroForm) Tj\nET\n";
    let page_id = create_simple_page(content, &mut doc);

    // Create pages dict
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    let pages_id = doc.add_object(pages_dict);

    // Update page parent
    if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Create read-only text field
    let mut field1_dict = Dictionary::new();
    field1_dict.set("T", Object::String(b"company_name".to_vec(), false));
    field1_dict.set("FT", Object::Name(b"Tx".to_vec()));
    field1_dict.set("V", Object::String(b"Acme Corporation".to_vec(), false));
    field1_dict.set("Ff", Object::Integer(1)); // ReadOnly flag (bit 0)
    field1_dict.set("Rect", Object::Array(vec![
        Object::Real(100.0), Object::Real(650.0),
        Object::Real(300.0), Object::Real(670.0)
    ]));
    let field1_id = doc.add_object(field1_dict);

    // Create pre-filled but not read-only field
    let mut field2_dict = Dictionary::new();
    field2_dict.set("T", Object::String(b"contact_email".to_vec(), false));
    field2_dict.set("FT", Object::Name(b"Tx".to_vec()));
    field2_dict.set("V", Object::String(b"contact@example.com".to_vec(), false));
    field2_dict.set("Ff", Object::Integer(0)); // Not read-only
    field2_dict.set("Rect", Object::Array(vec![
        Object::Real(100.0), Object::Real(620.0),
        Object::Real(300.0), Object::Real(640.0)
    ]));
    let field2_id = doc.add_object(field2_dict);

    // Create read-only checkbox
    let mut field3_dict = Dictionary::new();
    field3_dict.set("T", Object::String(b"verified".to_vec(), false));
    field3_dict.set("FT", Object::Name(b"Btn".to_vec()));
    field3_dict.set("V", Object::Name(b"Yes".to_vec()));
    field3_dict.set("Ff", Object::Integer(1)); // ReadOnly flag
    field3_dict.set("Rect", Object::Array(vec![
        Object::Real(100.0), Object::Real(590.0),
        Object::Real(115.0), Object::Real(605.0)
    ]));
    let field3_id = doc.add_object(field3_dict);

    // Create AcroForm dict
    let mut acroform_dict = Dictionary::new();
    acroform_dict.set("Fields", Object::Array(vec![
        Object::Reference(field1_id),
        Object::Reference(field2_id),
        Object::Reference(field3_id),
    ]));
    let acroform_id = doc.add_object(acroform_dict);

    // Create catalog
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));
    catalog_dict.set("AcroForm", Object::Reference(acroform_id));

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Save PDF
    let bytes = doc.save_to_bytes().unwrap();
    let mut file = File::create("tests/fixtures/forms/acroform-readonly.pdf").unwrap();
    file.write_all(&bytes).unwrap();
    println!("Created acroform-readonly.pdf");

    // Create ground truth JSON
    let ground_truth = r#"{
  "form_fields": [
    {
      "name": "company_name",
      "field_type": "Tx",
      "value": "Acme Corporation",
      "default_value": null,
      "flags": {
        "read_only": true,
        "required": false,
        "multiline": false,
        "password": false
      },
      "max_length": null
    },
    {
      "name": "contact_email",
      "field_type": "Tx",
      "value": "contact@example.com",
      "default_value": null,
      "flags": {
        "read_only": false,
        "required": false,
        "multiline": false,
        "password": false
      },
      "max_length": null
    },
    {
      "name": "verified",
      "field_type": "Btn",
      "value": "Yes",
      "default_value": null,
      "flags": {
        "read_only": true,
        "required": false,
        "radio": false,
        "pushbutton": false
      },
      "button_kind": "checkbox",
      "checked": true
    }
  ]
}"#;

    let mut json_file = File::create("tests/fixtures/forms/acroform-readonly.json").unwrap();
    json_file.write_all(ground_truth.as_bytes()).unwrap();
    println!("Created acroform-readonly.json");
}

fn create_acroform_submit_pdf() {
    let mut doc = Document::with_version("1.4");

    // Create page
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(AcroForm with Submit Button) Tj\nET\n";
    let page_id = create_simple_page(content, &mut doc);

    // Create pages dict
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    let pages_id = doc.add_object(pages_dict);

    // Update page parent
    if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Create text field
    let mut field1_dict = Dictionary::new();
    field1_dict.set("T", Object::String(b"username".to_vec(), false));
    field1_dict.set("FT", Object::Name(b"Tx".to_vec()));
    field1_dict.set("V", Object::String(b"".to_vec(), false));
    field1_dict.set("Ff", Object::Integer(2)); // Required flag
    field1_dict.set("Rect", Object::Array(vec![
        Object::Real(100.0), Object::Real(650.0),
        Object::Real(300.0), Object::Real(670.0)
    ]));
    let field1_id = doc.add_object(field1_dict);

    // Create submit button
    let mut submit_dict = Dictionary::new();
    submit_dict.set("T", Object::String(b"submit".to_vec(), false));
    submit_dict.set("FT", Object::Name(b"Btn".to_vec()));
    submit_dict.set("Ff", Object::Integer(1 << 25)); // Pushbutton flag
    submit_dict.set("Rect", Object::Array(vec![
        Object::Real(100.0), Object::Real(600.0),
        Object::Real(200.0), Object::Real(620.0)
    ]));

    // Add submit action
    let mut action_dict = Dictionary::new();
    action_dict.set("Type", Object::Name(b"Action".to_vec()));
    action_dict.set("S", Object::Name(b"SubmitForm".to_vec()));
    action_dict.set("F", Object::String(b"https://example.com/submit".to_vec(), false));
    let action_id = doc.add_object(action_dict);
    submit_dict.set("A", Object::Reference(action_id));

    let submit_id = doc.add_object(submit_dict);

    // Create reset button
    let mut reset_dict = Dictionary::new();
    reset_dict.set("T", Object::String(b"reset".to_vec(), false));
    reset_dict.set("FT", Object::Name(b"Btn".to_vec()));
    reset_dict.set("Ff", Object::Integer(1 << 25)); // Pushbutton flag
    reset_dict.set("Rect", Object::Array(vec![
        Object::Real(220.0), Object::Real(600.0),
        Object::Real(320.0), Object::Real(620.0)
    ]));

    // Add reset action
    let mut reset_action_dict = Dictionary::new();
    reset_action_dict.set("Type", Object::Name(b"Action".to_vec()));
    reset_action_dict.set("S", Object::Name(b"ResetForm".to_vec()));
    let reset_action_id = doc.add_object(reset_action_dict);
    reset_dict.set("A", Object::Reference(reset_action_id));

    let reset_id = doc.add_object(reset_dict);

    // Create AcroForm dict
    let mut acroform_dict = Dictionary::new();
    acroform_dict.set("Fields", Object::Array(vec![
        Object::Reference(field1_id),
        Object::Reference(submit_id),
        Object::Reference(reset_id),
    ]));
    let acroform_id = doc.add_object(acroform_dict);

    // Create catalog
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));
    catalog_dict.set("AcroForm", Object::Reference(acroform_id));

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Save PDF
    let bytes = doc.save_to_bytes().unwrap();
    let mut file = File::create("tests/fixtures/forms/acroform-submit.pdf").unwrap();
    file.write_all(&bytes).unwrap();
    println!("Created acroform-submit.pdf");

    // Create ground truth JSON
    let ground_truth = r#"{
  "form_fields": [
    {
      "name": "username",
      "field_type": "Tx",
      "value": "",
      "default_value": null,
      "flags": {
        "read_only": false,
        "required": true,
        "multiline": false,
        "password": false
      },
      "max_length": null
    },
    {
      "name": "submit",
      "field_type": "Btn",
      "value": null,
      "default_value": null,
      "flags": {
        "read_only": false,
        "required": false,
        "radio": false,
        "pushbutton": true
      },
      "button_kind": "pushbutton",
      "action": {
        "type": "SubmitForm",
        "url": "https://example.com/submit"
      }
    },
    {
      "name": "reset",
      "field_type": "Btn",
      "value": null,
      "default_value": null,
      "flags": {
        "read_only": false,
        "required": false,
        "radio": false,
        "pushbutton": true
      },
      "button_kind": "pushbutton",
      "action": {
        "type": "ResetForm"
      }
    }
  ]
}"#;

    let mut json_file = File::create("tests/fixtures/forms/acroform-submit.json").unwrap();
    json_file.write_all(ground_truth.as_bytes()).unwrap();
    println!("Created acroform-submit.json");
}

fn create_xfa_dynamic_pdf() {
    let mut doc = Document::with_version("1.7");

    // Create page
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(XFA Dynamic Form Placeholder) Tj\nET\n";
    let page_id = create_simple_page(content, &mut doc);

    // Create pages dict
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    let pages_id = doc.add_object(pages_dict);

    // Update page parent
    if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Create minimal XFA data (placeholder - actual XFA requires XML streams)
    let xfa_data = b"<?xml version=\"1.0\"?>
<xdp:xdp xmlns:xdp=\"http://ns.adobe.com/xdp/\">
  <xfa:datasets xmlns:xfa=\"http://ns.adobe.com/xfa/\" />
  <template xmlns=\"http://www.xfa.org/schema/xfa-template/2.8/\" />
</xdp:xdp>";

    let xfa_stream_id = doc.new_object_id();
    doc.objects.insert(xfa_stream_id, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        xfa_data.to_vec()
    )));

    // Create simple AcroForm field as fallback
    let mut field1_dict = Dictionary::new();
    field1_dict.set("T", Object::String(b"xfa_field1".to_vec(), false));
    field1_dict.set("FT", Object::Name(b"Tx".to_vec()));
    field1_dict.set("V", Object::String(b"XFA Value".to_vec(), false));
    field1_dict.set("Rect", Object::Array(vec![
        Object::Real(100.0), Object::Real(650.0),
        Object::Real(300.0), Object::Real(670.0)
    ]));
    let field1_id = doc.add_object(field1_dict);

    // Create AcroForm dict with XFA reference
    let mut acroform_dict = Dictionary::new();
    acroform_dict.set("Fields", Object::Array(vec![
        Object::Reference(field1_id),
    ]));
    acroform_dict.set("XFA", Object::Array(vec![
        Object::Reference(xfa_stream_id),
    ]));
    let acroform_id = doc.add_object(acroform_dict);

    // Create catalog
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));
    catalog_dict.set("AcroForm", Object::Reference(acroform_id));

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Save PDF
    let bytes = doc.save_to_bytes().unwrap();
    let mut file = File::create("tests/fixtures/forms/xfa-dynamic.pdf").unwrap();
    file.write_all(&bytes).unwrap();
    println!("Created xfa-dynamic.pdf");

    // Create ground truth JSON (minimal for now)
    let ground_truth = r#"{
  "form_fields": [
    {
      "name": "xfa_field1",
      "field_type": "Tx",
      "value": "XFA Value",
      "default_value": null,
      "flags": {
        "read_only": false,
        "required": false,
        "multiline": false,
        "password": false
      },
      "max_length": null
    }
  ],
  "note": "XFA XML parsing not yet implemented - this fixture tests XFA detection"
}"#;

    let mut json_file = File::create("tests/fixtures/forms/xfa-dynamic.json").unwrap();
    json_file.write_all(ground_truth.as_bytes()).unwrap();
    println!("Created xfa-dynamic.json");
}

fn main() {
    println!("Generating AcroForm and XFA test fixtures...");

    create_acroform_text_fields_pdf();
    create_acroform_readonly_pdf();
    create_acroform_submit_pdf();
    create_xfa_dynamic_pdf();

    println!("\nAll form fixtures generated successfully!");
    println!("\nFixtures created:");
    println!("  - tests/fixtures/forms/acroform-text-fields.pdf");
    println!("  - tests/fixtures/forms/acroform-readonly.pdf");
    println!("  - tests/fixtures/forms/acroform-submit.pdf");
    println!("  - tests/fixtures/forms/xfa-dynamic.pdf");
}
