//! Echantillonnage LTTB (Largest-Triangle-Three-Buckets, Sveinn Steinarsson)
//! pour les graphes a beaucoup de points (M6-3, CA : "echantillonnage LTTB
//! au-dela de 5000 points" — G2 notamment, un point par main). Algorithme
//! generique, sans dependance a un crate externe (aucun disponible deja
//! dans le workspace) : garde toujours le premier et le dernier point
//! exacts, et choisit dans chaque compartiment intermediaire le point qui
//! forme le plus grand triangle avec le point deja retenu et la moyenne du
//! compartiment suivant — préserve la forme visuelle de la courbe mieux
//! qu'un sous-echantillonnage uniforme.

/// Reduit `points` (deja tries par `x` croissant) a `threshold` points au
/// maximum. Renvoie `points` inchange si `threshold == 0`, `threshold >= 3`
/// n'est pas respecte, ou s'il y a deja moins de points que `threshold`
/// (rien a faire).
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn lttb(points: &[(f64, f64)], threshold: usize) -> Vec<(f64, f64)> {
    if threshold < 3 || points.len() <= threshold {
        return points.to_vec();
    }

    let mut sampled = Vec::with_capacity(threshold);
    sampled.push(points[0]);

    // Taille de compartiment pour les points intermediaires (hors premier
    // et dernier, geres a part).
    #[allow(clippy::cast_precision_loss)]
    let every = (points.len() - 2) as f64 / (threshold - 2) as f64;

    let mut a = 0usize;
    for i in 0..(threshold - 2) {
        #[allow(clippy::cast_precision_loss)]
        let avg_range_start = ((i + 1) as f64 * every) as usize + 1;
        #[allow(clippy::cast_precision_loss)]
        let avg_range_end = (((i + 2) as f64 * every) as usize + 1).min(points.len());
        let avg_range = &points[avg_range_start..avg_range_end];
        #[allow(clippy::cast_precision_loss)]
        let avg_len = avg_range.len() as f64;
        let (avg_x, avg_y) = avg_range
            .iter()
            .fold((0.0, 0.0), |(sx, sy), (x, y)| (sx + x, sy + y));
        let (avg_x, avg_y) = (avg_x / avg_len, avg_y / avg_len);

        #[allow(clippy::cast_precision_loss)]
        let range_start = (i as f64 * every) as usize + 1;
        #[allow(clippy::cast_precision_loss)]
        let range_end = ((i + 1) as f64 * every) as usize + 1;

        let (ax, ay) = points[a];
        let mut max_area = -1.0f64;
        let mut max_area_point = points[range_start.min(points.len() - 1)];
        let mut next_a = range_start;

        for (j, &(x, y)) in points
            .iter()
            .enumerate()
            .take(range_end.min(points.len()))
            .skip(range_start)
        {
            let area = ((ax - avg_x) * (y - ay) - (ax - x) * (avg_y - ay)).abs() * 0.5;
            if area > max_area {
                max_area = area;
                max_area_point = (x, y);
                next_a = j;
            }
        }

        sampled.push(max_area_point);
        a = next_a;
    }

    sampled.push(points[points.len() - 1]);
    sampled
}

#[cfg(test)]
mod tests {
    use super::lttb;

    #[test]
    fn returns_the_input_unchanged_when_it_already_fits_under_the_threshold() {
        let points = vec![(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)];
        assert_eq!(lttb(&points, 5000), points);
    }

    #[test]
    fn returns_the_input_unchanged_for_a_threshold_below_three() {
        let points: Vec<(f64, f64)> = (0..100).map(|i| (f64::from(i), f64::from(i))).collect();
        assert_eq!(lttb(&points, 2), points);
    }

    #[test]
    fn downsamples_to_exactly_the_requested_threshold() {
        let points: Vec<(f64, f64)> = (0..10_000).map(|i| (f64::from(i), f64::from(i))).collect();
        let sampled = lttb(&points, 500);
        assert_eq!(sampled.len(), 500);
    }

    #[test]
    fn always_keeps_the_first_and_last_point_exactly() {
        let points: Vec<(f64, f64)> = (0..1000)
            .map(|i| (f64::from(i), (f64::from(i) * 0.37).sin()))
            .collect();
        let sampled = lttb(&points, 50);
        assert_eq!(sampled.first(), points.first());
        assert_eq!(sampled.last(), points.last());
    }

    #[test]
    fn preserves_a_single_sharp_spike_in_an_otherwise_flat_series() {
        // Une serie plate avec un seul pic tres marque : LTTB doit le
        // retenir (plus grande aire de triangle possible), contrairement a
        // un sous-echantillonnage uniforme qui l'ecraserait la plupart du
        // temps.
        let mut points: Vec<(f64, f64)> = (0..300).map(|i| (f64::from(i), 0.0)).collect();
        points[150] = (150.0, 1000.0);

        let sampled = lttb(&points, 30);
        assert!(
            sampled
                .iter()
                .any(|&(x, y)| (x - 150.0).abs() < 1e-9 && (y - 1000.0).abs() < 1e-9),
            "le pic devrait survivre a l'echantillonnage : {sampled:?}"
        );
    }
}
