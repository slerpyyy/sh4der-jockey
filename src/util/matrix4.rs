#[derive(std::fmt::Debug)]
pub struct Matrix4 {
    pub elements: [[f32; 4]; 4],
}

impl Matrix4 {
    pub fn new(elements: [[f32; 4]; 4]) -> Self {
        return Matrix4 { elements };
    }

    pub fn identity() -> Self {
        return Matrix4 {
            elements: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };
    }

    pub fn elements_flattened(&self) -> [f32; 16] {
        return [
            self.elements[0][0],
            self.elements[0][1],
            self.elements[0][2],
            self.elements[0][3],

            self.elements[1][0],
            self.elements[1][1],
            self.elements[1][2],
            self.elements[1][3],

            self.elements[2][0],
            self.elements[2][1],
            self.elements[2][2],
            self.elements[2][3],

            self.elements[3][0],
            self.elements[3][1],
            self.elements[3][2],
            self.elements[3][3],
        ];
    }

    /// Multiply this Matrix4 by another Matrix4.
    pub fn multiply(&self, matrix: Matrix4) -> Matrix4 {
        let a = self.elements;
        let b = matrix.elements;

        return Matrix4::new([
            [
                a[0][0] * b[0][0] + a[1][0] * b[0][1] + a[2][0] * b[0][2] + a[3][0] * b[0][3],
                a[0][1] * b[0][0] + a[1][1] * b[0][1] + a[2][1] * b[0][2] + a[3][1] * b[0][3],
                a[0][2] * b[0][0] + a[1][2] * b[0][1] + a[2][2] * b[0][2] + a[3][2] * b[0][3],
                a[0][3] * b[0][0] + a[1][3] * b[0][1] + a[2][3] * b[0][2] + a[3][3] * b[0][3],
            ],
            [
                a[0][0] * b[1][0] + a[1][0] * b[1][1] + a[2][0] * b[1][2] + a[3][0] * b[1][3],
                a[0][1] * b[1][0] + a[1][1] * b[1][1] + a[2][1] * b[1][2] + a[3][1] * b[1][3],
                a[0][2] * b[1][0] + a[1][2] * b[1][1] + a[2][2] * b[1][2] + a[3][2] * b[1][3],
                a[0][3] * b[1][0] + a[1][3] * b[1][1] + a[2][3] * b[1][2] + a[3][3] * b[1][3],
            ],
            [
                a[0][0] * b[2][0] + a[1][0] * b[2][1] + a[2][0] * b[2][2] + a[3][0] * b[2][3],
                a[0][1] * b[2][0] + a[1][1] * b[2][1] + a[2][1] * b[2][2] + a[3][1] * b[2][3],
                a[0][2] * b[2][0] + a[1][2] * b[2][1] + a[2][2] * b[2][2] + a[3][2] * b[2][3],
                a[0][3] * b[2][0] + a[1][3] * b[2][1] + a[2][3] * b[2][2] + a[3][3] * b[2][3],
            ],
            [
                a[0][0] * b[3][0] + a[1][0] * b[3][1] + a[2][0] * b[3][2] + a[3][0] * b[3][3],
                a[0][1] * b[3][0] + a[1][1] * b[3][1] + a[2][1] * b[3][2] + a[3][1] * b[3][3],
                a[0][2] * b[3][0] + a[1][2] * b[3][1] + a[2][2] * b[3][2] + a[3][2] * b[3][3],
                a[0][3] * b[3][0] + a[1][3] * b[3][1] + a[2][3] * b[3][2] + a[3][3] * b[3][3],
            ],
        ]);
    }

    /// Return an inverse of this matrix.
    ///
    /// Yoinked from Three.js (MIT)
    /// https://github.com/mrdoob/three.js/blob/master/LICENSE
    #[allow(dead_code)]
    pub fn invert(&self) -> Matrix4 {
        let m = self.elements;
        let a00 = m[0][0];
        let a01 = m[0][1];
        let a02 = m[0][2];
        let a03 = m[0][3];
        let a10 = m[1][0];
        let a11 = m[1][1];
        let a12 = m[1][2];
        let a13 = m[1][3];
        let a20 = m[2][0];
        let a21 = m[2][1];
        let a22 = m[2][2];
        let a23 = m[2][3];
        let a30 = m[3][0];
        let a31 = m[3][1];
        let a32 = m[3][2];
        let a33 = m[3][3];

        let b00 = a00 * a11 - a01 * a10;
        let b01 = a00 * a12 - a02 * a10;
        let b02 = a00 * a13 - a03 * a10;
        let b03 = a01 * a12 - a02 * a11;
        let b04 = a01 * a13 - a03 * a11;
        let b05 = a02 * a13 - a03 * a12;
        let b06 = a20 * a31 - a21 * a30;
        let b07 = a20 * a32 - a22 * a30;
        let b08 = a20 * a33 - a23 * a30;
        let b09 = a21 * a32 - a22 * a31;
        let b10 = a21 * a33 - a23 * a31;
        let b11 = a22 * a33 - a23 * a32;

        let det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;

        if det == 0.0 {
            return Matrix4::new([
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
            ]);
        }

        let inv_det = 1.0 / det;

        Matrix4::new([
            [
                inv_det * (a11 * b11 - a12 * b10 + a13 * b09),
                inv_det * (a02 * b10 - a01 * b11 - a03 * b09),
                inv_det * (a31 * b05 - a32 * b04 + a33 * b03),
                inv_det * (a22 * b04 - a21 * b05 - a23 * b03),
            ],
            [
                inv_det * (a12 * b08 - a10 * b11 - a13 * b07),
                inv_det * (a00 * b11 - a02 * b08 + a03 * b07),
                inv_det * (a32 * b02 - a30 * b05 - a33 * b01),
                inv_det * (a20 * b05 - a22 * b02 + a23 * b01),
            ],
            [
                inv_det * (a10 * b10 - a11 * b08 + a13 * b06),
                inv_det * (a01 * b08 - a00 * b10 - a03 * b06),
                inv_det * (a30 * b04 - a31 * b02 + a33 * b00),
                inv_det * (a21 * b02 - a20 * b04 - a23 * b00),
            ],
            [
                inv_det * (a11 * b07 - a10 * b09 - a12 * b06),
                inv_det * (a00 * b09 - a01 * b07 + a02 * b06),
                inv_det * (a31 * b01 - a30 * b03 - a32 * b00),
                inv_det * (a20 * b03 - a21 * b01 + a22 * b00),
            ]
        ])
    }

    /// Return a transpose of this matrix.
    #[allow(dead_code)]
    pub fn transpose(&self) -> Matrix4 {
        let m = self.elements;

        Matrix4::new([
            [m[0][0], m[1][0], m[2][0], m[3][0]],
            [m[0][1], m[1][1], m[2][1], m[3][1]],
            [m[0][2], m[1][2], m[2][2], m[3][2]],
            [m[0][3], m[1][3], m[2][3], m[3][3]],
        ])
    }
}

impl Clone for Matrix4 {
    fn clone(&self) -> Self {
        Matrix4::new(self.elements.clone())
    }
}

impl Copy for Matrix4 {}

#[cfg(test)]
mod test {
    use nearly_eq::assert_nearly_eq;

    use super::*;

    #[test]
    fn multiply() {
        let mat_a = Matrix4::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.7071067811865476, 0.7071067811865475, 0.0],
            [0.0, -0.7071067811865475, 0.7071067811865476, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let mat_b = Matrix4::new([
            [0.8574929257125442, 0.0, -0.5144957554275265, 0.0],
            [-0.2910427500435996, 0.824621125123532, -0.48507125007266594, 0.0],
            [0.4242640687119285, 0.565685424949238, 0.7071067811865475, 0.0],
            [3.0, 4.0, 5.0, 1.0],
        ]);
        let subject = mat_b.multiply(mat_a);
        let expected = Matrix4::new([
            [0.8574929257125442, 0.0, -0.5144957554275266, 0.0],
            [0.09420169782898935, 0.9830951894845299, 0.1570028297149822, 0.0],
            [0.5057983021710106, -0.18309518948452996, 0.8429971702850176, 0.0],
            [3.0, 4.0, 5.0, 1.0]
        ]);

        assert_nearly_eq!(subject.elements, expected.elements);
    }

    #[test]
    fn invert() {
        let source = Matrix4::new([
            [0.8574929257125442, 0.0, -0.5144957554275265, 0.0],
            [-0.2910427500435996, 0.824621125123532, -0.48507125007266594, 0.0],
            [0.4242640687119285, 0.565685424949238, 0.7071067811865475, 0.0],
            [3.0, 4.0, 5.0, 1.0],
        ]);
        let subject = source.invert();
        let expected = Matrix4::new([
            [0.8574929257125443, -0.2910427500435996, 0.42426406871192857, 0.0],
            [0.0, 0.8246211251235323, 0.5656854249492381, 0.0],
            [-0.5144957554275266, -0.48507125007266605, 0.7071067811865476, 0.0],
            [2.2204460492503136e-16, 4.440892098500627e-16, -7.071067811865476, 1.0]
        ]);

        assert_nearly_eq!(subject.elements, expected.elements);
    }

    #[test]
    fn transpose() {
        let source = Matrix4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);
        let subject = source.transpose();
        let expected = Matrix4::new([
            [1.0, 5.0, 9.0, 13.0],
            [2.0, 6.0, 10.0, 14.0],
            [3.0, 7.0, 11.0, 15.0],
            [4.0, 8.0, 12.0, 16.0],
        ]);

        assert_nearly_eq!(subject.elements, expected.elements);
    }
}
