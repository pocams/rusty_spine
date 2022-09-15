use std::{ffi::CString, sync::Arc};

use crate::{
    bone::Bone,
    c::{spSkeleton, spSkeleton_create, spSkeleton_findBone, spSkeleton_updateWorldTransform},
    error::Error,
    skeleton_data::SkeletonData,
    sync_ptr::SyncPtr,
};

#[derive(Debug)]
pub struct Skeleton {
    c_skeleton: SyncPtr<spSkeleton>,
    _skeleton_data: Arc<SkeletonData>,
    bones: Vec<Bone>,
}

impl Skeleton {
    pub fn new(skeleton_data: Arc<SkeletonData>) -> Result<Self, Error> {
        let c_skeleton = unsafe { spSkeleton_create(skeleton_data.c_ptr()) };
        let mut bones = vec![];
        let bone_count = unsafe { (*c_skeleton).bonesCount };
        for i in 0..bone_count {
            unsafe {
                bones.push(Bone::new(*(*c_skeleton).bones.offset(i as isize)));
            }
        }
        Ok(Self {
            c_skeleton: SyncPtr(c_skeleton),
            _skeleton_data: skeleton_data,
            bones,
        })
    }

    pub fn update_world_transform(&mut self) {
        unsafe {
            spSkeleton_updateWorldTransform(self.c_skeleton.0);
        }
    }

    pub fn bones(&self) -> &Vec<Bone> {
        &self.bones
    }

    pub fn bones_mut(&mut self) -> &mut Vec<Bone> {
        &mut self.bones
    }

    pub fn find_bone(&self, name: &str) -> Option<&Bone> {
        if let Ok(c_name) = CString::new(name) {
            let bone = unsafe { spSkeleton_findBone(self.c_skeleton.0, c_name.as_ptr()) };
            if !bone.is_null() {
                unsafe { self.bones.get((*(*bone).data).index as usize) }
            } else {
                None
            }
        } else {
            None
        }
    }

    c_ptr!(c_skeleton, spSkeleton);
}