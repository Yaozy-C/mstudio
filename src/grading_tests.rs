use crate::grading::Grade;
#[test]
fn neutral_grade_preserves_rgb_and_exposure_is_monotonic() {
    let grade = Grade::default();
    grade.validate().unwrap();
    for r in 0..11 {
        for g in 0..11 {
            for b in 0..11 {
                let p = [r as f64 / 10., g as f64 / 10., b as f64 / 10.];
                let q = grade.pixel(p);
                assert!(p.into_iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-8));
            }
        }
    }
    let brighter = Grade {
        exposure: 1.,
        ..grade.clone()
    };
    assert!(brighter.pixel([0.3; 3])[0] > grade.pixel([0.3; 3])[0]);
}
#[test]
fn controls_validate_and_lut_cache_reuses_recipe() {
    let grade: Grade = serde_json::from_str(
        r#"{"exposure":0.5,"shadows":30,"wheels":[[220,12,0],[0,0,0],[40,10,0]]}"#,
    )
    .unwrap();
    grade.validate().unwrap();
    let lut = crate::grade_lut::bake(&grade).unwrap();
    let stamp = std::fs::metadata(&lut).unwrap().modified().unwrap();
    assert_eq!(crate::grade_lut::bake(&grade).unwrap(), lut);
    assert_eq!(std::fs::metadata(&lut).unwrap().modified().unwrap(), stamp);
    let text = std::fs::read_to_string(&lut).unwrap();
    assert_eq!(text.lines().count(), 4 + 33 * 33 * 33);
    let mut invalid = grade;
    invalid.curves[0] = [0.8, 0.5, 0.2];
    assert!(invalid.validate().is_err());
    invalid.curves = Grade::default().curves;
    invalid.hsl[0][0] = f64::NAN;
    assert!(invalid.validate().is_err());
}
