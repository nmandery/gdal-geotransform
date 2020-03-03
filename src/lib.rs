#[cfg(test)]
#[macro_use]
extern crate approx;

use std::convert::TryFrom;

use gdal::raster::dataset::GeoTransform;
use geo_types::{Coordinate, Rect};


fn rect_from_coordinates(c1: Coordinate<f64>, c2: Coordinate<f64>) -> Rect<f64> {
    Rect::new(
        Coordinate {
            x: if c1.x > c2.x { c2.x } else { c1.x },
            y: if c1.y > c2.y { c2.y } else { c1.y },
        },
        Coordinate {
            x: if c1.x < c2.x { c2.x } else { c1.x },
            y: if c1.y < c2.y { c2.y } else { c1.y },
        },
    )
}

#[derive(Clone)]
pub struct GeoTransformer {
    geotransform: GeoTransform,
    inv_geotransform: GeoTransform,
}

impl GeoTransformer {
    /// Convert a coordinate to the pixel coordinate in the dataset.
    ///
    /// Will return pixel coordinates outside of the bounds of the dataset when
    /// the coordinates are outside of the envelope of the raster.
    pub fn coordinate_to_pixel(&self, coordinate: Coordinate<f64>) -> (usize, usize) {
        // ported from https://github.com/OSGeo/gdal/blob/master/gdal/apps/gdallocationinfo.cpp#L282
        (
            (self.inv_geotransform[0] + (self.inv_geotransform[1] * coordinate.x) + (self.inv_geotransform[2] * coordinate.y)).floor() as usize,
            (self.inv_geotransform[3] + (self.inv_geotransform[4] * coordinate.x) + (self.inv_geotransform[5] * coordinate.y)).floor() as usize
        )
    }

    /// Convert a pixel coordinate to the geo-coordinate
    pub fn pixel_to_coordinate(&self, pixel: (usize, usize)) -> Coordinate<f64> {
        // ported form https://github.com/OSGeo/gdal/blob/18bfbd32302f611bde0832f61ca0747d4c4421dd/gdal/apps/gdalinfo_lib.cpp#L1443
        Coordinate {
            x: self.geotransform[0] + (self.geotransform[1] * pixel.0 as f64) + (self.geotransform[2] * pixel.1 as f64),
            y: self.geotransform[3] + (self.geotransform[4] * pixel.0 as f64) + (self.geotransform[5] * pixel.1 as f64),
        }
    }

    /// generate to boundingbox from the size of a gdal dataset
    pub fn bounds_from_size(&self, size: (usize, usize)) -> Rect<f64> {
        let c1 = self.pixel_to_coordinate((0, 0));
        let c2 = self.pixel_to_coordinate(size);
        rect_from_coordinates(c1, c2)
    }
}

impl TryFrom<GeoTransform> for GeoTransformer {
    type Error = &'static str;

    fn try_from(geotransform: GeoTransform) -> Result<Self, Self::Error> {
        let mut inv_geotransform = GeoTransform::default();
        let mut gt = geotransform;
        let res = unsafe { gdal_sys::GDALInvGeoTransform(gt.as_mut_ptr(), inv_geotransform.as_mut_ptr()) };
        if res == 0 {
            Err("Could not invert geotransform")
        } else {
            Ok(GeoTransformer { geotransform: gt, inv_geotransform })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;
    use std::path::Path;

    use gdal::raster::Dataset;
    use geo_types::Coordinate;

    use crate::GeoTransformer;

    macro_rules! assert_coordinates_relative_eq {
        ($given:expr, $expected:expr) => {
            assert_relative_eq!($given.x, $expected.x, epsilon = 0.000001);
            assert_relative_eq!($given.y, $expected.y, epsilon = 0.000001);
        }
    }


    fn open_dataset(dataset_filename: &str) -> (Dataset, GeoTransformer) {
        let path = Path::new(dataset_filename);
        let dataset = Dataset::open(path).unwrap();

        let geotransform = dataset.geo_transform().unwrap();
        let geotransformer = GeoTransformer::try_from(geotransform).unwrap();

        (dataset, geotransformer)
    }

    #[test]
    fn test_geotransformer_bounds() {
        let (dataset, geotransformer) = open_dataset("data/small.tiff");
        let bounds = geotransformer.bounds_from_size(dataset.size());

        // expected values where collected by using gdalinfo
        assert_coordinates_relative_eq!(bounds.min, Coordinate {
            x: 11.3610659,
            y: 32.2014463,
        });
        assert_coordinates_relative_eq!(bounds.max, Coordinate {
            x: 28.2838457,
            y: 46.2520256,
        });
    }


    #[test]
    fn test_geotransformer_coordinate_to_pixel() {
        let (dataset, geotransformer) = open_dataset("data/small.tiff");
        let bounds = geotransformer.bounds_from_size(dataset.size());

        let c1 = geotransformer.coordinate_to_pixel(bounds.min);
        assert_eq!(c1, (0, 44));
        let c2 = geotransformer.coordinate_to_pixel(bounds.max);
        assert_eq!(c2, (52, 0));
    }
}
