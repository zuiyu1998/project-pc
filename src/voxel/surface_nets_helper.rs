use bevy::math::{IVec3, Vec3A, Vec3Swizzles};
use ndshape::ConstShape;

use super::{ChunkPosition, Map, MeshShape, SdfValue};

pub const NULL_VERTEX: u32 = u32::MAX;

#[derive(Default, Clone)]
pub struct SurfaceNetsBuffer {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// The triangle mesh indices.
    pub indices: Vec<u32>,

    pub stride_to_index: Vec<u32>,

    /// Local 3D array coordinates of every voxel that intersects the isosurface.
    pub surface_points: Vec<[u32; 3]>,
    /// Stride of every voxel that intersects the isosurface. Can be used for efficient post-processing.
    pub surface_strides: Vec<u32>,
}

impl SurfaceNetsBuffer {
    pub fn reset(&mut self) {
        self.stride_to_index.resize(MeshShape::USIZE, NULL_VERTEX);
    }
}

impl SurfaceNetsBuffer {}

pub struct SurfaceNetsHelper<'a> {
    map: &'a Map,
    chunk_position: &'a ChunkPosition,
}

impl<'a> SurfaceNetsHelper<'a> {
    pub fn new(map: &'a Map, chunk_position: &'a ChunkPosition) -> Self {
        SurfaceNetsHelper {
            map,
            chunk_position,
        }
    }

    pub fn get_real_sdf_value(&self, mesh_position: IVec3) -> SdfValue {
        let position: IVec3 = self.chunk_position.get_relative_position(mesh_position);

        Map::get_sdf_value(self.map.noise.seed, position)
    }

    pub fn surface_nets(&mut self, output: &mut SurfaceNetsBuffer) {
        self.estimate_surface(output);

        self.make_all_quads(output);
    }

    // For every edge that crosses the isosurface, make a quad between the "centers" of the four cubes touching that surface. The
    // "centers" are actually the vertex positions found earlier. Also make sure the triangles are facing the right way. See the
    // comments on `maybe_make_quad` to help with understanding the indexing.
    fn make_all_quads(&self, output: &mut SurfaceNetsBuffer) {
        let xyz_strides = [
            MeshShape::linearize([1, 0, 0]) as usize,
            MeshShape::linearize([0, 1, 0]) as usize,
            MeshShape::linearize([0, 0, 1]) as usize,
        ];

        for (&[x, y, z], &p_stride) in output
            .surface_points
            .iter()
            .zip(output.surface_strides.iter())
        {
            let p_stride = p_stride as usize;

            // Do edges parallel with the X axis
            if y != 0 && z != 0 && x != MeshShape::ARRAY[0] - 1 {
                self.maybe_make_quad(
                    &output.stride_to_index,
                    &output.positions,
                    p_stride,
                    p_stride + xyz_strides[0],
                    xyz_strides[1],
                    xyz_strides[2],
                    &mut output.indices,
                );
            }
            // Do edges parallel with the Y axis
            if x != 0 && z != 0 && y != MeshShape::ARRAY[1] - 1 {
                self.maybe_make_quad(
                    &output.stride_to_index,
                    &output.positions,
                    p_stride,
                    p_stride + xyz_strides[1],
                    xyz_strides[2],
                    xyz_strides[0],
                    &mut output.indices,
                );
            }
            // Do edges parallel with the Z axis
            if x != 0 && y != 0 && z != MeshShape::ARRAY[2] - 1 {
                self.maybe_make_quad(
                    &output.stride_to_index,
                    &output.positions,
                    p_stride,
                    p_stride + xyz_strides[2],
                    xyz_strides[0],
                    xyz_strides[1],
                    &mut output.indices,
                );
            }
        }
    }

    fn maybe_make_quad(
        &self,
        stride_to_index: &[u32],
        positions: &[[f32; 3]],
        p1: usize,
        p2: usize,
        axis_b_stride: usize,
        axis_c_stride: usize,
        indices: &mut Vec<u32>,
    ) {
        let p1_p = MeshShape::delinearize(p1 as u32);

        let d1 =
            self.get_real_sdf_value(IVec3::new(p1_p[0] as i32, p1_p[1] as i32, p1_p[2] as i32));
        let p2_p = MeshShape::delinearize(p2 as u32);

        let d2 =
            self.get_real_sdf_value(IVec3::new(p2_p[0] as i32, p2_p[1] as i32, p2_p[2] as i32));

        let negative_face = match (d1.is_empty(), d2.is_empty()) {
            (true, false) => false,
            (false, true) => true,
            _ => return, // No face.
        };

        // The triangle points, viewed face-front, look like this:
        // v1 v3
        // v2 v4
        let v1 = stride_to_index[p1];
        let v2 = stride_to_index[p1 - axis_b_stride];
        let v3 = stride_to_index[p1 - axis_c_stride];
        let v4 = stride_to_index[p1 - axis_b_stride - axis_c_stride];
        let (pos1, pos2, pos3, pos4) = (
            Vec3A::from(positions[v1 as usize]),
            Vec3A::from(positions[v2 as usize]),
            Vec3A::from(positions[v3 as usize]),
            Vec3A::from(positions[v4 as usize]),
        );
        // Split the quad along the shorter axis, rather than the longer one.
        let quad = if pos1.distance_squared(pos4) < pos2.distance_squared(pos3) {
            if negative_face {
                [v1, v4, v2, v1, v3, v4]
            } else {
                [v1, v2, v4, v1, v4, v3]
            }
        } else if negative_face {
            [v2, v3, v4, v2, v1, v3]
        } else {
            [v2, v4, v3, v2, v3, v1]
        };
        indices.extend_from_slice(&quad);
    }

    pub fn estimate_surface(&self, output: &mut SurfaceNetsBuffer) {
        //遍历chunk的每个采集点
        for z in 0..MeshShape::ARRAY[2] {
            for y in 0..MeshShape::ARRAY[1] {
                for x in 0..MeshShape::ARRAY[0] {
                    let p = IVec3::new(x as i32, y as i32, z as i32);
                    let stride = MeshShape::linearize([x, y, z]);

                    if self.estimate_surface_in_cube(p, output) {
                        output.stride_to_index[stride as usize] = output.positions.len() as u32 - 1;

                        output.surface_points.push([x, y, z]);
                        output.surface_strides.push(stride);
                    } else {
                        output.stride_to_index[stride as usize] = NULL_VERTEX;
                    }
                }
            }
        }
    }

    // Consider the grid-aligned cube where `p` is the minimal corner. Find a point inside this cube that is approximately on the
    // isosurface.
    //
    // This is done by estimating, for each cube edge, where the isosurface crosses the edge (if it does at all). Then the estimated
    // surface point is the average of these edge crossings.
    fn estimate_surface_in_cube(&self, p: IVec3, output: &mut SurfaceNetsBuffer) -> bool {
        // Get the signed distance values at each corner of this cube.
        let mut corner_dists = [0f32; 8];
        let mut num_negative = 0;
        for (i, dist) in corner_dists.iter_mut().enumerate() {
            let lp = p + IVec3::new(
                CUBE_CORNERS[i][0] as i32,
                CUBE_CORNERS[i][1] as i32,
                CUBE_CORNERS[i][2] as i32,
            );

            let s = self.get_real_sdf_value(lp);

            *dist = s.value;
            if s.is_empty() {
                num_negative += 1;
            }
        }

        //如果没有等势点经过直接返回
        if num_negative == 0 || num_negative == 8 {
            // No crossings.
            return false;
        }

        let c = centroid_of_edge_intersections(&corner_dists);

        let p = Vec3A::from([p.x as f32, p.y as f32, p.z as f32]);

        //插入一个顶点
        output.positions.push((p + c).into());
        //插入发现
        output.normals.push(sdf_gradient(&corner_dists, c).into());

        true
    }
}

/// Calculate the normal as the gradient of the distance field. Don't bother making it a unit vector, since we'll do that on the
/// GPU.
///
/// For each dimension, there are 4 cube edges along that axis. This will do bilinear interpolation between the differences
/// along those edges based on the position of the surface (s).
fn sdf_gradient(dists: &[f32; 8], s: Vec3A) -> Vec3A {
    let p00 = Vec3A::from([dists[0b001], dists[0b010], dists[0b100]]);
    let n00 = Vec3A::from([dists[0b000], dists[0b000], dists[0b000]]);

    let p10 = Vec3A::from([dists[0b101], dists[0b011], dists[0b110]]);
    let n10 = Vec3A::from([dists[0b100], dists[0b001], dists[0b010]]);

    let p01 = Vec3A::from([dists[0b011], dists[0b110], dists[0b101]]);
    let n01 = Vec3A::from([dists[0b010], dists[0b100], dists[0b001]]);

    let p11 = Vec3A::from([dists[0b111], dists[0b111], dists[0b111]]);
    let n11 = Vec3A::from([dists[0b110], dists[0b101], dists[0b011]]);

    // Each dimension encodes an edge delta, giving 12 in total.
    let d00 = p00 - n00; // Edges (0b00x, 0b0y0, 0bz00)
    let d10 = p10 - n10; // Edges (0b10x, 0b0y1, 0bz10)
    let d01 = p01 - n01; // Edges (0b01x, 0b1y0, 0bz01)
    let d11 = p11 - n11; // Edges (0b11x, 0b1y1, 0bz11)

    let neg = Vec3A::ONE - s;

    // Do bilinear interpolation between 4 edges in each dimension.
    neg.yzx() * neg.zxy() * d00
        + neg.yzx() * s.zxy() * d10
        + s.yzx() * neg.zxy() * d01
        + s.yzx() * s.zxy() * d11
}

fn centroid_of_edge_intersections(dists: &[f32; 8]) -> Vec3A {
    let mut count = 0;
    let mut sum = Vec3A::ZERO;
    for &[corner1, corner2] in CUBE_EDGES.iter() {
        let d1 = dists[corner1 as usize];
        let d2 = dists[corner2 as usize];

        //如果边上有等势点
        if (d1 < 0.0) != (d2 < 0.0) {
            count += 1;
            sum += estimate_surface_edge_intersection(corner1, corner2, d1, d2);
        }
    }

    sum / count as f32
}

fn estimate_surface_edge_intersection(
    corner1: u32,
    corner2: u32,
    value1: f32,
    value2: f32,
) -> Vec3A {
    let interp1 = value1 / (value1 - value2);
    let interp2 = 1.0 - interp1;

    interp2 * CUBE_CORNER_VECTORS[corner1 as usize]
        + interp1 * CUBE_CORNER_VECTORS[corner2 as usize]
}

const CUBE_CORNER_VECTORS: [Vec3A; 8] = [
    Vec3A::from_array([0.0, 0.0, 0.0]),
    Vec3A::from_array([1.0, 0.0, 0.0]),
    Vec3A::from_array([0.0, 1.0, 0.0]),
    Vec3A::from_array([1.0, 1.0, 0.0]),
    Vec3A::from_array([0.0, 0.0, 1.0]),
    Vec3A::from_array([1.0, 0.0, 1.0]),
    Vec3A::from_array([0.0, 1.0, 1.0]),
    Vec3A::from_array([1.0, 1.0, 1.0]),
];

const CUBE_EDGES: [[u32; 2]; 12] = [
    [0b000, 0b001],
    [0b000, 0b010],
    [0b000, 0b100],
    [0b001, 0b011],
    [0b001, 0b101],
    [0b010, 0b011],
    [0b010, 0b110],
    [0b011, 0b111],
    [0b100, 0b101],
    [0b100, 0b110],
    [0b101, 0b111],
    [0b110, 0b111],
];

//任何一个点的
const CUBE_CORNERS: [[u32; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [0, 1, 0],
    [1, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [0, 1, 1],
    [1, 1, 1],
];
