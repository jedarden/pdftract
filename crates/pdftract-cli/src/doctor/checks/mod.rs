// Individual check modules
mod binary;
mod cache_dir;
#[cfg(feature = "ocr")]
mod leptonica;
#[cfg(feature = "ocr")]
mod libopenjp2;
#[cfg(feature = "ocr")]
mod libtiff;
mod locale;
mod memory;
#[cfg(feature = "remote")]
mod network;
#[cfg(feature = "full-render")]
mod pdfium;
#[cfg(feature = "profiles")]
mod profile_path;
mod temp_dir;
#[cfg(feature = "ocr")]
mod tesseract;
#[cfg(feature = "ocr")]
mod tesseract_langs;
#[cfg(unix)]
mod ulimit;

use super::Check;

/// Registry of all available checks
pub mod registry {
    use super::*;

    pub fn all_checks() -> Vec<Box<dyn Check>> {
        let mut checks: Vec<Box<dyn Check>> = vec![
            Box::new(binary::BinaryCheck),
            Box::new(cache_dir::CacheDirCheck),
            Box::new(memory::MemoryCheck),
            Box::new(locale::LocaleCheck),
            Box::new(temp_dir::TempDirCheck),
        ];

        #[cfg(feature = "ocr")]
        {
            checks.extend([
                Box::new(tesseract::TesseractCheck) as Box<dyn Check>,
                Box::new(tesseract_langs::TesseractLangsCheck) as Box<dyn Check>,
                Box::new(leptonica::LeptonicaCheck) as Box<dyn Check>,
                Box::new(libtiff::LibtiffCheck) as Box<dyn Check>,
                Box::new(libopenjp2::Libopenjp2Check) as Box<dyn Check>,
            ]);
        }

        #[cfg(feature = "full-render")]
        {
            checks.push(Box::new(pdfium::PdfiumCheck) as Box<dyn Check>);
        }

        #[cfg(feature = "remote")]
        {
            checks.push(Box::new(network::NetworkCheck) as Box<dyn Check>);
        }

        #[cfg(feature = "profiles")]
        {
            checks.push(Box::new(profile_path::ProfilePathCheck) as Box<dyn Check>);
        }

        #[cfg(unix)]
        {
            checks.push(Box::new(ulimit::UlimitCheck) as Box<dyn Check>);
        }

        checks
    }
}
