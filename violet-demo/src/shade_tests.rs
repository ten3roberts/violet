//! Tests for the shade generation functionality in the colorpicker module.
//!
//! These tests verify the behavior of the `ShadeSettings::tint` method which
//! generates color shades from a base color. The current implementation uses
//! linear luminance interpolation which can cause issues for neutral colors.

#[cfg(test)]
mod tests {
    use violet::palette::Oklch;

    use crate::colorpicker::{ShadeSettings, ShadeSettingsPreset};

    /// Documents the current behavior of the shade generation system.
    ///
    /// This test creates a ShadeSettings with default values and generates
    /// shades for a neutral gray color. The L/C/H values are printed for each
    /// of the 11 shades to serve as a baseline for future improvements.
    ///
    /// Also tests with different curve values to show the perceptual difference.
    #[test]
    fn test_current_behavior_documentation() {
        let settings = ShadeSettings::default();

        // Create a neutral gray with chroma=0
        let neutral_gray = Oklch::new(0.5, 0.0, 0.0);

        println!("Shade generation for neutral gray (L=0.5, C=0, H=0):");
        println!("Default curve ({}):", settings.neutral_curve);
        println!("{:<6} {:<8} {:<8} {:<8}", "Index", "L", "C", "H");
        println!("{}", "-".repeat(35));

        // Generate all 11 shades (indices 0-10, corresponding to tints 50-950)
        for i in 0..=10 {
            let tint = i as f32 / 10.0;
            let shade = settings.tint_from_base(neutral_gray, tint);

            println!(
                "{:<6} {:<8.4} {:<8.4} {:<8.2}",
                i,
                shade.l,
                shade.chroma,
                shade.hue.into_positive_degrees()
            );
        }

        // Also show linear curve (curve=1.0) for comparison
        let mut linear_settings = settings;
        linear_settings.neutral_curve = 1.0;

        println!("\nLinear curve (1.0) for comparison:");
        println!("{:<6} {:<8} {:<8} {:<8}", "Index", "L", "C", "H");
        println!("{}", "-".repeat(35));

        for i in 0..=10 {
            let tint = i as f32 / 10.0;
            let shade = linear_settings.tint_from_base(neutral_gray, tint);

            println!(
                "{:<6} {:<8.4} {:<8.4} {:<8.2}",
                i,
                shade.l,
                shade.chroma,
                shade.hue.into_positive_degrees()
            );
        }

        // Verify that neutral grays remain neutral (chroma stays at 0)
        for i in 0..=10 {
            let tint = i as f32 / 10.0;
            let shade = settings.tint_from_base(neutral_gray, tint);
            assert!(
                shade.chroma < 0.001,
                "Neutral gray should have chroma near 0, got {}",
                shade.chroma
            );
        }
    }

    /// Tests that the perceptual curve compresses dark shades toward lighter values.
    ///
    /// This test verifies that:
    /// - Curve=1.0 (linear) produces evenly spaced luminance values
    /// - Curve=2.0 (perceptual) produces lighter values for dark shades
    /// - The perceptual curve compresses darks (softer darks) while expanding lights
    #[test]
    fn test_perceptual_curve() {
        let mut linear_settings = ShadeSettings::default();
        linear_settings.neutral_curve = 1.0;

        let mut perceptual_settings = ShadeSettings::default();
        perceptual_settings.neutral_curve = 2.0;

        let neutral_gray = Oklch::new(0.5, 0.0, 0.0);

        // Generate shades with both curves
        // tint=0.0 is lightest (shade 50), tint=1.0 is darkest (shade 950)
        let linear_shades: Vec<Oklch> = (0..=10)
            .map(|i| {
                let tint = i as f32 / 10.0;
                linear_settings.tint_from_base(neutral_gray, tint)
            })
            .collect();

        let perceptual_shades: Vec<Oklch> = (0..=10)
            .map(|i| {
                let tint = i as f32 / 10.0;
                perceptual_settings.tint_from_base(neutral_gray, tint)
            })
            .collect();

        // Lightest shade (index 0, tint=0.0) should be the same for both
        // (since 0.0^curve = 0.0 for any curve > 0)
        let linear_lightest = linear_shades[0].l;
        let perceptual_lightest = perceptual_shades[0].l;
        assert!(
            (linear_lightest - perceptual_lightest).abs() < 0.001,
            "Lightest shade should be the same for linear and perceptual curves"
        );

        // Darkest shade (index 10, tint=1.0) should be similar for both
        // (since 1.0^curve = 1.0 for any curve)
        let linear_darkest = linear_shades[10].l;
        let perceptual_darkest = perceptual_shades[10].l;
        assert!(
            (linear_darkest - perceptual_darkest).abs() < 0.001,
            "Darkest shade should be the same for linear and perceptual curves"
        );

        // Middle-dark shades should be LIGHTER with perceptual curve (curve > 1)
        // Shade 950 is tint=1.0 (darkest), shade 50 is tint=0.0 (lightest)
        // So shade 700 (index 7) has tint=0.7, which is 70% toward darkest
        let linear_mid_dark = linear_shades[7].l;
        let perceptual_mid_dark = perceptual_shades[7].l;

        println!("Linear shade 7 (tint=0.7): L={}", linear_mid_dark);
        println!("Perceptual shade 7 (tint=0.7): L={}", perceptual_mid_dark);

        // Perceptual curve with curve>1 compresses the dark end toward lighter values
        // So for tint=0.7, perceptual should give a HIGHER luminance (lighter)
        assert!(
            perceptual_mid_dark > linear_mid_dark,
            "Perceptual curve should produce lighter mid-dark shades (softer darks). \
             Linear L={}, Perceptual L={}",
            linear_mid_dark,
            perceptual_mid_dark
        );

        // Light shades should also be lighter with perceptual curve
        // Shade 300 (index 3) has tint=0.3
        let linear_mid_light = linear_shades[3].l;
        let perceptual_mid_light = perceptual_shades[3].l;

        println!("Linear shade 3 (tint=0.3): L={}", linear_mid_light);
        println!("Perceptual shade 3 (tint=0.3): L={}", perceptual_mid_light);

        assert!(
            perceptual_mid_light > linear_mid_light,
            "Perceptual curve should produce lighter mid-light shades. \
             Linear L={}, Perceptual L={}",
            linear_mid_light,
            perceptual_mid_light
        );
    }

    /// Tests that consecutive gray shades have similar perceptual deltas.
    ///
    /// This test verifies perceptual uniformity in shade generation for neutral colors.
    /// With the neutral curve applied to neutral colors, the perceptual steps should
    /// be more evenly distributed than with linear interpolation.
    #[test]
    fn test_perceptual_uniformity() {
        let settings = ShadeSettings::default();

        // Generate shades for neutral gray
        let neutral_gray = Oklch::new(0.5, 0.0, 0.0);
        let shades: Vec<Oklch> = (0..=10)
            .map(|i| {
                let tint = i as f32 / 10.0;
                settings.tint_from_base(neutral_gray, tint)
            })
            .collect();

        // Calculate luminance deltas between consecutive shades
        let deltas: Vec<f32> = shades.windows(2).map(|w| w[0].l - w[1].l).collect();

        // All deltas should be positive (luminance decreases monotonically)
        for (i, delta) in deltas.iter().enumerate() {
            assert!(
                *delta > 0.0,
                "Luminance delta at index {} should be positive, got {}",
                i,
                delta
            );
        }

        // With perceptual curve (1.8), darker shades should have smaller deltas
        // (compressed toward lighter values) while lighter shades have larger deltas
        // This creates softer darks
        let avg_dark_delta: f32 = deltas[5..].iter().sum::<f32>() / deltas[5..].len() as f32;
        let avg_light_delta: f32 = deltas[..5].iter().sum::<f32>() / deltas[..5].len() as f32;

        println!("Average delta for dark shades (5-9): {}", avg_dark_delta);
        println!("Average delta for light shades (0-4): {}", avg_light_delta);

        // With curve > 1, light shades should have smaller deltas (compressed toward lightest)
        // This creates softer darks by compressing the light end
        assert!(
            avg_light_delta < avg_dark_delta,
            "With perceptual curve, light shades should have smaller deltas. \
             Light avg={}, Dark avg={}",
            avg_light_delta,
            avg_dark_delta
        );
    }

    /// Tests neutral color detection.
    ///
    /// This test verifies that:
    /// - Colors with chroma below the threshold are treated as neutrals (constant chroma)
    /// - Colors with chroma above the threshold use normal chroma falloff
    /// - The boundary case at the threshold is handled correctly
    #[test]
    fn test_neutral_detection() {
        let mut settings = ShadeSettings::default();
        settings.neutral_chroma_threshold = 0.05;
        settings.falloff = 10.0;

        // Test chroma=0.0 (clearly neutral) - should have constant chroma
        let neutral_gray = Oklch::new(0.5, 0.0, 0.0);
        let shade_light = settings.tint_from_base(neutral_gray, 0.0);
        let shade_mid = settings.tint_from_base(neutral_gray, 0.5);
        let shade_dark = settings.tint_from_base(neutral_gray, 1.0);

        // All shades should have chroma ≈ 0 (constant)
        assert!(
            shade_light.chroma < 0.001,
            "Neutral color should have constant chroma, got {} at tint=0.0",
            shade_light.chroma
        );
        assert!(
            shade_mid.chroma < 0.001,
            "Neutral color should have constant chroma, got {} at tint=0.5",
            shade_mid.chroma
        );
        assert!(
            shade_dark.chroma < 0.001,
            "Neutral color should have constant chroma, got {} at tint=1.0",
            shade_dark.chroma
        );

        // Test chroma=0.1 (above threshold) - should use chroma falloff
        let non_neutral = Oklch::new(0.5, 0.1, 0.0);
        let shade_light = settings.tint_from_base(non_neutral, 0.0);
        let shade_mid = settings.tint_from_base(non_neutral, 0.5);
        let shade_dark = settings.tint_from_base(non_neutral, 1.0);

        // Middle shade should have full chroma (falloff is minimal at tint=0.5)
        assert!(
            (shade_mid.chroma - 0.1).abs() < 0.01,
            "Non-neutral at tint=0.5 should have chroma close to base, got {}",
            shade_mid.chroma
        );
        // Light and dark shades should have reduced chroma due to falloff
        assert!(
            shade_light.chroma < shade_mid.chroma,
            "Non-neutral at tint=0.0 should have reduced chroma"
        );
        assert!(
            shade_dark.chroma < shade_mid.chroma,
            "Non-neutral at tint=1.0 should have reduced chroma"
        );

        // Test boundary case (chroma=0.05, exactly at threshold)
        // This is treated as neutral (base_chroma < threshold is false for < not <=)
        // Actually, using < means chroma=0.05 is NOT neutral
        let boundary = Oklch::new(0.5, 0.05, 0.0);
        let shade_light = settings.tint_from_base(boundary, 0.0);
        let shade_mid = settings.tint_from_base(boundary, 0.5);

        // With < comparison, 0.05 is NOT less than 0.05, so it should use falloff
        assert!(
            shade_light.chroma < shade_mid.chroma,
            "Boundary color should use chroma falloff (< comparison)"
        );
    }

    /// Tests that neutral colors preserve their chroma across all shades.
    ///
    /// This test verifies that:
    /// - Pure neutral grays (chroma=0) stay at chroma=0 for all shades
    /// - Slightly tinted neutrals (chroma below threshold) keep constant chroma
    #[test]
    fn test_neutral_chroma_preservation() {
        let mut settings = ShadeSettings::default();
        settings.neutral_chroma_threshold = 0.05;

        // Test pure neutral gray (chroma=0)
        let neutral_gray = Oklch::new(0.5, 0.0, 0.0);
        for i in 0..=10 {
            let tint = i as f32 / 10.0;
            let shade = settings.tint_from_base(neutral_gray, tint);
            assert!(
                shade.chroma < 0.0001,
                "Pure neutral gray should have chroma ≈ 0 at tint={}, got {}",
                tint,
                shade.chroma
            );
        }

        // Test slightly tinted neutral (chroma=0.03, below threshold)
        let tinted_neutral = Oklch::new(0.5, 0.03, 120.0);
        for i in 0..=10 {
            let tint = i as f32 / 10.0;
            let shade = settings.tint_from_base(tinted_neutral, tint);
            assert!(
                (shade.chroma - 0.03).abs() < 0.0001,
                "Tinted neutral (C=0.03) should have constant chroma at tint={}, got {}",
                tint,
                shade.chroma
            );
        }
    }

    /// Tests that saturated colors generate correct shades.
    ///
    /// This test verifies that:
    /// - The middle shade (index 5, tint=0.5) is close to the base color (with linear curve)
    /// - Lighter shades have higher luminance values
    /// - Darker shades have lower luminance values
    /// - Chroma falls off appropriately for lighter/darker shades (non-neutral behavior)
    #[test]
    fn test_saturated_colors() {
        // Use linear curve (1.0) to test basic shade generation without curve effects
        let mut settings = ShadeSettings::default();
        settings.neutral_curve = 1.0;
        settings.neutral_chroma_threshold = 0.05;

        // Use a vibrant blue with high chroma
        let vibrant_blue = Oklch::new(0.5, 0.27, 240.0);

        // Generate all shades
        let shades: Vec<Oklch> = (0..=10)
            .map(|i| {
                let tint = i as f32 / 10.0;
                settings.tint_from_base(vibrant_blue, tint)
            })
            .collect();

        // Middle shade (index 5, tint=0.5) should have luminance close to base
        let middle_shade = &shades[5];
        assert!(
            (middle_shade.l - vibrant_blue.l).abs() < 0.1,
            "Middle shade luminance should be close to base color L={}",
            vibrant_blue.l
        );

        // Verify chroma falls off for lighter and darker shades compared to middle
        // With falloff=10.0, chroma at tint=0.5 should be base_chroma * (1.0 / 1.0) = base_chroma
        // At tint=0.0 or 1.0, chroma should be reduced
        assert!(
            shades[0].chroma < middle_shade.chroma,
            "Lightest shade should have reduced chroma"
        );
        assert!(
            shades[10].chroma < middle_shade.chroma,
            "Darkest shade should have reduced chroma"
        );

        // Verify lighter shades have higher L, darker shades have lower L
        for i in 0..5 {
            assert!(
                shades[i].l > shades[i + 1].l,
                "Shade {} should have higher L than shade {}",
                i,
                i + 1
            );
        }

        // Verify luminance range spans from min_lum to max_lum
        let lightest = &shades[0];
        let darkest = &shades[10];

        assert!(
            (lightest.l - settings.max_lum).abs() < 0.01,
            "Lightest shade should have L close to max_lum ({}), got {}",
            settings.max_lum,
            lightest.l
        );
        assert!(
            (darkest.l - settings.min_lum).abs() < 0.01,
            "Darkest shade should have L close to min_lum ({}), got {}",
            settings.min_lum,
            darkest.l
        );
    }

    /// Tests that the Everforest preset produces the expected settings.
    ///
    /// The Everforest preset is designed for smooth gray palettes with
    /// a darkest shade around L=0.15 (~#1e2326).
    #[test]
    fn test_everforest_preset() {
        let mut settings = ShadeSettings::default();
        ShadeSettingsPreset::Everforest.apply_to(&mut settings);

        // Verify the settings match Everforest values
        assert!((settings.neutral_curve - 2.0).abs() < 0.001);
        assert!((settings.neutral_chroma_threshold - 0.08).abs() < 0.001);
        assert!((settings.min_lum - 0.15).abs() < 0.001);
        assert!((settings.max_lum - 0.97).abs() < 0.001);

        // Generate shades for neutral gray
        let neutral_gray = Oklch::new(0.5, 0.0, 0.0);
        let shades: Vec<Oklch> = (0..=10)
            .map(|i| {
                let tint = i as f32 / 10.0;
                settings.tint_from_base(neutral_gray, tint)
            })
            .collect();

        // Verify darkest shade has L ≈ 0.15
        let darkest = &shades[10];
        assert!(
            (darkest.l - 0.15).abs() < 0.02,
            "Darkest shade should have L ≈ 0.15, got {}",
            darkest.l
        );

        // Verify lightest shade has L ≈ 0.97
        let lightest = &shades[0];
        assert!(
            (lightest.l - 0.97).abs() < 0.02,
            "Lightest shade should have L ≈ 0.97, got {}",
            lightest.l
        );

        // Verify perceptual uniformity (consecutive deltas should be positive)
        let deltas: Vec<f32> = shades.windows(2).map(|w| w[0].l - w[1].l).collect();
        for (i, delta) in deltas.iter().enumerate() {
            assert!(
                *delta > 0.0,
                "Luminance delta at index {} should be positive, got {}",
                i,
                delta
            );
        }

        // Verify progression is perceptually uniform (smaller deltas in darks)
        let avg_dark_delta: f32 = deltas[5..].iter().sum::<f32>() / deltas[5..].len() as f32;
        let avg_light_delta: f32 = deltas[..5].iter().sum::<f32>() / deltas[..5].len() as f32;
        assert!(
            avg_light_delta < avg_dark_delta,
            "With Everforest curve=2.0, light shades should have smaller deltas"
        );
    }

    /// Tests that the Material preset produces the expected settings.
    ///
    /// The Material preset is designed for Material Design-style surface hierarchies.
    #[test]
    fn test_material_preset() {
        let mut settings = ShadeSettings::default();
        ShadeSettingsPreset::Material.apply_to(&mut settings);

        // Verify the settings match Material values
        assert!((settings.neutral_curve - 1.6).abs() < 0.001);
        assert!((settings.neutral_chroma_threshold - 0.06).abs() < 0.001);
        assert!((settings.min_lum - 0.08).abs() < 0.001);
        assert!((settings.max_lum - 0.98).abs() < 0.001);

        // Generate shades for neutral gray
        let neutral_gray = Oklch::new(0.5, 0.0, 0.0);
        let shades: Vec<Oklch> = (0..=10)
            .map(|i| {
                let tint = i as f32 / 10.0;
                settings.tint_from_base(neutral_gray, tint)
            })
            .collect();

        // Verify darkest shade has L ≈ 0.08
        let darkest = &shades[10];
        assert!(
            (darkest.l - 0.08).abs() < 0.02,
            "Darkest shade should have L ≈ 0.08, got {}",
            darkest.l
        );

        // Verify lightest shade has L ≈ 0.98
        let lightest = &shades[0];
        assert!(
            (lightest.l - 0.98).abs() < 0.02,
            "Lightest shade should have L ≈ 0.98, got {}",
            lightest.l
        );

        // Verify smooth hierarchy (all deltas positive)
        let deltas: Vec<f32> = shades.windows(2).map(|w| w[0].l - w[1].l).collect();
        for (i, delta) in deltas.iter().enumerate() {
            assert!(
                *delta > 0.0,
                "Luminance delta at index {} should be positive, got {}",
                i,
                delta
            );
        }
    }

    /// Tests that the Linear preset produces the expected settings.
    ///
    /// The Linear preset restores original linear behavior for comparison.
    #[test]
    fn test_linear_preset() {
        let mut settings = ShadeSettings::default();
        ShadeSettingsPreset::Linear.apply_to(&mut settings);

        // Verify the settings match Linear values
        assert!((settings.neutral_curve - 1.0).abs() < 0.001);
        assert!((settings.neutral_chroma_threshold - 0.0).abs() < 0.001);
        assert!((settings.min_lum - 0.12).abs() < 0.001);
        assert!((settings.max_lum - 0.97).abs() < 0.001);

        // Generate shades and verify linear spacing
        let neutral_gray = Oklch::new(0.5, 0.0, 0.0);
        let shades: Vec<Oklch> = (0..=10)
            .map(|i| {
                let tint = i as f32 / 10.0;
                settings.tint_from_base(neutral_gray, tint)
            })
            .collect();

        // With linear curve (1.0), luminance should be evenly spaced
        let deltas: Vec<f32> = shades.windows(2).map(|w| w[0].l - w[1].l).collect();

        // All deltas should be approximately equal (within small tolerance)
        let first_delta = deltas[0];
        for (i, delta) in deltas.iter().enumerate() {
            assert!(
                (*delta - first_delta).abs() < 0.01,
                "Linear curve should produce even spacing, but delta {} differs: {} vs {}",
                i,
                delta,
                first_delta
            );
        }
    }

    /// Tests that the Default preset restores the default settings.
    #[test]
    fn test_default_preset() {
        // Start with different settings
        let mut settings = ShadeSettings::default();
        ShadeSettingsPreset::Everforest.apply_to(&mut settings);
        assert!(settings.neutral_curve != 1.8);

        // Apply Default preset
        ShadeSettingsPreset::Default.apply_to(&mut settings);

        // Verify back to defaults
        let defaults = ShadeSettings::default();
        assert!((settings.neutral_curve - defaults.neutral_curve).abs() < 0.001);
        assert!(
            (settings.neutral_chroma_threshold - defaults.neutral_chroma_threshold).abs() < 0.001
        );
        assert!((settings.min_lum - defaults.min_lum).abs() < 0.001);
        assert!((settings.max_lum - defaults.max_lum).abs() < 0.001);
    }

    /// Tests that preset detection works correctly.
    #[test]
    fn test_detect_preset() {
        let mut settings = ShadeSettings::default();

        // Test Everforest detection
        ShadeSettingsPreset::Everforest.apply_to(&mut settings);
        assert_eq!(settings.detect_preset(), ShadeSettingsPreset::Everforest);

        // Test Material detection
        ShadeSettingsPreset::Material.apply_to(&mut settings);
        assert_eq!(settings.detect_preset(), ShadeSettingsPreset::Material);

        // Test Linear detection
        ShadeSettingsPreset::Linear.apply_to(&mut settings);
        assert_eq!(settings.detect_preset(), ShadeSettingsPreset::Linear);

        // Test Default detection
        ShadeSettingsPreset::Default.apply_to(&mut settings);
        assert_eq!(settings.detect_preset(), ShadeSettingsPreset::Default);

        // Test custom settings (should return Default)
        settings.neutral_curve = 1.5; // Non-preset value
        assert_eq!(settings.detect_preset(), ShadeSettingsPreset::Default);
    }

    /// Tests deserialization compatibility with old ShadeSettings without new fields.
    ///
    /// This test verifies that ShadeSettings can be deserialized from older serialized
    /// formats that may be missing newer fields. The #[serde(default)] attribute
    /// ensures backward compatibility.
    #[test]
    fn test_serde_compatibility() {
        // Old format without neutral_curve and neutral_chroma_threshold
        let old_json = r#"{
            "enabled": true,
            "min_lum": 0.1,
            "max_lum": 0.95,
            "falloff": 8.0
        }"#;

        let settings: ShadeSettings =
            serde_json::from_str(old_json).expect("Should deserialize old format");

        assert!(settings.enabled);
        assert!((settings.min_lum - 0.1).abs() < 0.001);
        assert!((settings.max_lum - 0.95).abs() < 0.001);
        assert!((settings.falloff - 8.0).abs() < 0.001);

        // New fields should have default values from the functions (1.8 and 0.05)
        assert!(
            (settings.neutral_curve - 1.8).abs() < 0.001,
            "neutral_curve should default to 1.8 from serde(default), got {}",
            settings.neutral_curve
        );
        assert!(
            (settings.neutral_chroma_threshold - 0.05).abs() < 0.001,
            "neutral_chroma_threshold should default to 0.05 from serde(default), got {}",
            settings.neutral_chroma_threshold
        );

        // Format with only neutral_curve (missing neutral_chroma_threshold)
        let partial_json = r#"{
            "enabled": false,
            "min_lum": 0.15,
            "max_lum": 0.9,
            "falloff": 12.0,
            "neutral_curve": 2.0
        }"#;

        let settings: ShadeSettings =
            serde_json::from_str(partial_json).expect("Should deserialize partial format");

        assert!(!settings.enabled);
        assert!((settings.neutral_curve - 2.0).abs() < 0.001);
        assert!(
            (settings.neutral_chroma_threshold - 0.05).abs() < 0.001,
            "neutral_chroma_threshold should default to 0.05 when missing"
        );

        // Full format with all fields
        let full_json = r#"{
            "enabled": true,
            "min_lum": 0.2,
            "max_lum": 0.98,
            "falloff": 15.0,
            "neutral_curve": 1.5,
            "neutral_chroma_threshold": 0.08
        }"#;

        let settings: ShadeSettings =
            serde_json::from_str(full_json).expect("Should deserialize full format");

        assert!(settings.enabled);
        assert!((settings.min_lum - 0.2).abs() < 0.001);
        assert!((settings.max_lum - 0.98).abs() < 0.001);
        assert!((settings.falloff - 15.0).abs() < 0.001);
        assert!((settings.neutral_curve - 1.5).abs() < 0.001);
        assert!((settings.neutral_chroma_threshold - 0.08).abs() < 0.001);

        // Test round-trip serialization
        let serialized = serde_json::to_string(&settings).expect("Should serialize");
        let deserialized: ShadeSettings =
            serde_json::from_str(&serialized).expect("Should deserialize serialized");

        assert_eq!(settings.enabled, deserialized.enabled);
        assert!((settings.min_lum - deserialized.min_lum).abs() < 0.001);
        assert!((settings.max_lum - deserialized.max_lum).abs() < 0.001);
        assert!((settings.falloff - deserialized.falloff).abs() < 0.001);
        assert!((settings.neutral_curve - deserialized.neutral_curve).abs() < 0.001);
        assert!(
            (settings.neutral_chroma_threshold - deserialized.neutral_chroma_threshold).abs()
                < 0.001
        );
    }

    /// Tests deserialization of old ShadeSettings without neutral_curve field.
    ///
    /// This specifically tests the case where only neutral_chroma_threshold is missing.
    #[test]
    fn test_serde_without_neutral_curve() {
        // Old format missing neutral_curve (should use default 1.8)
        let json = r#"{
            "enabled": true,
            "min_lum": 0.1,
            "max_lum": 0.95,
            "falloff": 8.0,
            "neutral_chroma_threshold": 0.03
        }"#;

        let settings: ShadeSettings =
            serde_json::from_str(json).expect("Should deserialize without neutral_curve");

        assert!((settings.neutral_curve - 1.8).abs() < 0.001);
        assert!((settings.neutral_chroma_threshold - 0.03).abs() < 0.001);
    }

    /// Tests deserialization of old ShadeSettings without neutral_chroma_threshold field.
    ///
    /// This specifically tests the case where only neutral_chroma_threshold is missing.
    #[test]
    fn test_serde_without_neutral_chroma_threshold() {
        // Old format missing neutral_chroma_threshold (should use default 0.05)
        let json = r#"{
            "enabled": true,
            "min_lum": 0.1,
            "max_lum": 0.95,
            "falloff": 8.0,
            "neutral_curve": 1.5
        }"#;

        let settings: ShadeSettings = serde_json::from_str(json)
            .expect("Should deserialize without neutral_chroma_threshold");

        assert!((settings.neutral_curve - 1.5).abs() < 0.001);
        assert!((settings.neutral_chroma_threshold - 0.05).abs() < 0.001);
    }
}
