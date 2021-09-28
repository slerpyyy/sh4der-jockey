use super::Matrix4;

#[derive(std::fmt::Debug)]
pub struct Matrix3 {
    pub elements: [[f32; 3]; 3],
}

impl Matrix3 {
    pub fn new(elements: [[f32; 3]; 3]) -> Self {
        return Matrix3 { elements };
    }

    #[allow(dead_code)]
    pub fn identity() -> Self {
        return Matrix3 {
            elements: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
        };
    }

    pub fn elements_flattened(&self) -> [f32; 9] {
        return [
            self.elements[0][0],
            self.elements[0][1],
            self.elements[0][2],

            self.elements[1][0],
            self.elements[1][1],
            self.elements[1][2],

            self.elements[2][0],
            self.elements[2][1],
            self.elements[2][2],
        ];
    }

    /// Multiply this Matrix3 by another Matrix3.
    #[allow(dead_code)]
    pub fn multiply(&self, matrix: Matrix3) -> Matrix3 {
        let a = self.elements;
        let b = matrix.elements;

        return Matrix3::new([
            [
                a[0][0] * b[0][0] + a[1][0] * b[0][1] + a[2][0] * b[0][2],
                a[0][1] * b[0][0] + a[1][1] * b[0][1] + a[2][1] * b[0][2],
                a[0][2] * b[0][0] + a[1][2] * b[0][1] + a[2][2] * b[0][2],
            ],
            [
                a[0][0] * b[1][0] + a[1][0] * b[1][1] + a[2][0] * b[1][2],
                a[0][1] * b[1][0] + a[1][1] * b[1][1] + a[2][1] * b[1][2],
                a[0][2] * b[1][0] + a[1][2] * b[1][1] + a[2][2] * b[1][2],
            ],
            [
                a[0][0] * b[2][0] + a[1][0] * b[2][1] + a[2][0] * b[2][2],
                a[0][1] * b[2][0] + a[1][1] * b[2][1] + a[2][1] * b[2][2],
                a[0][2] * b[2][0] + a[1][2] * b[2][1] + a[2][2] * b[2][2],
            ],
        ]);
    }

    /// Return an inverse of this matrix.
    ///
    /// Yoinked from Three.js (MIT)
    /// https://github.com/mrdoob/three.js/blob/master/LICENSE
    pub fn invert(&self) -> Matrix3 {
        let m = self.elements;
        let n11 = m[0][0];
        let n21 = m[0][1];
        let n31 = m[0][2];
        let n12 = m[1][0];
        let n22 = m[1][1];
        let n32 = m[1][2];
        let n13 = m[2][0];
        let n23 = m[2][1];
        let n33 = m[2][2];

        let t11 = n33 * n22 - n32 * n23;
        let t12 = n32 * n13 - n33 * n12;
        let t13 = n23 * n12 - n22 * n13;

        let det = n11 * t11 + n21 * t12 + n31 * t13;

        if det == 0.0 {
            return Matrix3::new([
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
            ]);
        }

        let inv_det = 1.0 / det;

        Matrix3::new([
            [
                inv_det * t11,
                inv_det * ( n31 * n23 - n33 * n21 ),
                inv_det * ( n32 * n21 - n31 * n22 ),
            ],
            [
                inv_det * t12,
                inv_det * ( n33 * n11 - n31 * n13 ),
                inv_det * ( n31 * n12 - n32 * n11 ),
            ],
            [
                inv_det * t13,
                inv_det * ( n21 * n13 - n23 * n11 ),
                inv_det * ( n22 * n11 - n21 * n12 ),
            ],
        ])
    }

    /// Return a transpose of this matrix.
    pub fn transpose(&self) -> Matrix3 {
        let m = self.elements;

        Matrix3::new([
            [m[0][0], m[1][0], m[2][0]],
            [m[0][1], m[1][1], m[2][1]],
            [m[0][2], m[1][2], m[2][2]],
        ])
    }
}

impl Clone for Matrix3 {
    fn clone(&self) -> Self {
        Matrix3::new(self.elements.clone())
    }
}

impl Copy for Matrix3 {}

impl From<Matrix4> for Matrix3 {
    fn from(matrix4: Matrix4) -> Matrix3 {
        let m = matrix4.elements;

        Matrix3::new([
            [m[0][0], m[0][1], m[0][2]],
            [m[1][0], m[1][1], m[1][2]],
            [m[2][0], m[2][1], m[2][2]],
        ])
    }
}

#[cfg(test)]
mod test {
    use nearly_eq::assert_nearly_eq;

    use super::*;

    #[test]
    fn multiply() {
        let mat_a = Matrix3::new([
            [1.0, 0.0, 0.0],
            [0.0, 0.7071067811865476, 0.7071067811865475],
            [0.0, -0.7071067811865475, 0.7071067811865476],
        ]);
        let mat_b = Matrix3::new([
            [0.8574929257125442, 0.0, -0.5144957554275265],
            [-0.2910427500435996, 0.824621125123532, -0.48507125007266594],
            [0.4242640687119285, 0.565685424949238, 0.7071067811865475],
        ]);
        let subject = mat_b.multiply(mat_a);
        let expected = Matrix3::new([
            [0.8574929257125442, 0.0, -0.5144957554275266],
            [0.09420169782898935, 0.9830951894845299, 0.1570028297149822],
            [0.5057983021710106, -0.18309518948452996, 0.8429971702850176],
        ]);

        assert_nearly_eq!(subject.elements, expected.elements);
    }

    #[test]
    fn invert() {
        let source = Matrix3::new([
            [0.8574929257125442, 0.0, -0.5144957554275265],
            [-0.2910427500435996, 0.824621125123532, -0.48507125007266594],
            [0.4242640687119285, 0.565685424949238, 0.7071067811865475],
        ]);
        let subject = source.invert();
        let expected = Matrix3::new([
            [0.8574929257125443, -0.2910427500435996, 0.42426406871192857],
            [0.0, 0.8246211251235323, 0.5656854249492381],
            [-0.5144957554275266, -0.48507125007266605, 0.7071067811865476],
        ]);

        assert_nearly_eq!(subject.elements, expected.elements);
    }

    #[test]
    fn transpose() {
        let source = Matrix3::new([
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
        ]);
        let subject = source.transpose();
        let expected = Matrix3::new([
            [1.0, 4.0, 7.0],
            [2.0, 5.0, 8.0],
            [3.0, 6.0, 9.0],
        ]);

        assert_nearly_eq!(subject.elements, expected.elements);
    }

    #[test]
    fn from_matrix4() {
        let source = Matrix4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);
        let subject = Matrix3::from(source);
        let expected = Matrix3::new([
            [1.0, 2.0, 3.0],
            [5.0, 6.0, 7.0],
            [9.0, 10.0, 11.0],
        ]);

        assert_nearly_eq!(subject.elements, expected.elements);
    }
}
