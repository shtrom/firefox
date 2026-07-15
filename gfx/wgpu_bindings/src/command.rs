/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    client::Client, id, raw_string_to_string, server::Global, BindingCommand,
    ComputePassEncoderCommand, DebugCommand, FfiSlice, Message, RawString,
};
use std::{borrow::Cow, ffi};
use wgc::{
    command::{PassTimestampWrites, RenderPassColorAttachment, RenderPassDepthStencilAttachment},
    id::{CommandEncoderId, TextureViewId},
};
use wgt::{BufferAddress, BufferSize, Color, DynamicOffset, IndexFormat};

use serde::{Deserialize, Serialize};

/// A stream of commands for a render pass or compute pass.
///
/// This also contains side tables referred to by certain commands,
/// like dynamic offsets for [`SetBindGroup`] or string data for
/// [`InsertDebugMarker`].
///
/// Render passes use `Pass<RenderCommand>`, whereas compute
/// passes use `Pass<ComputeCommand>`.
///
/// [`SetBindGroup`]: RenderCommand::SetBindGroup
/// [`InsertDebugMarker`]: RenderCommand::InsertDebugMarker
#[doc(hidden)]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Pass<C> {
    pub label: Option<String>,

    /// The stream of commands.
    pub commands: Vec<C>,

    /// Dynamic offsets consumed by [`SetBindGroup`] commands in `commands`.
    ///
    /// Each successive `SetBindGroup` consumes the next
    /// [`num_dynamic_offsets`] values from this list.
    pub dynamic_offsets: Vec<wgt::DynamicOffset>,

    /// Strings used by debug instructions.
    ///
    /// Each successive [`PushDebugGroup`] or [`InsertDebugMarker`]
    /// instruction consumes the next `len` bytes from this vector.
    pub string_data: Vec<u8>,
}

#[derive(Deserialize, Serialize)]
pub struct RecordedRenderPass {
    base: Pass<RenderCommand>,
    color_attachments: Vec<Option<RenderPassColorAttachment>>,
    depth_stencil_attachment: Option<RenderPassDepthStencilAttachment<TextureViewId>>,
    timestamp_writes: Option<PassTimestampWrites>,
    occlusion_query_set_id: Option<id::QuerySetId>,
}

impl RecordedRenderPass {
    pub fn new(
        label: Option<String>,
        color_attachments: Vec<Option<RenderPassColorAttachment>>,
        depth_stencil_attachment: Option<RenderPassDepthStencilAttachment<TextureViewId>>,
        timestamp_writes: Option<PassTimestampWrites>,
        occlusion_query_set_id: Option<id::QuerySetId>,
    ) -> Self {
        Self {
            base: Pass {
                label,
                commands: Vec::new(),
                dynamic_offsets: Vec::new(),
                string_data: Vec::new(),
            },
            color_attachments,
            depth_stencil_attachment,
            timestamp_writes,
            occlusion_query_set_id,
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum RenderCommand {
    SetBindGroup {
        index: u32,
        num_dynamic_offsets: usize,
        bind_group_id: Option<id::BindGroupId>,
    },
    SetPipeline(id::RenderPipelineId),
    SetIndexBuffer {
        buffer_id: id::BufferId,
        index_format: wgt::IndexFormat,
        offset: BufferAddress,
        size: Option<BufferSize>,
    },
    SetVertexBuffer {
        slot: u32,
        buffer_id: Option<id::BufferId>,
        offset: BufferAddress,
        size: Option<BufferSize>,
    },
    SetBlendConstant(Color),
    SetStencilReference(u32),
    SetViewport {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        depth_min: f32,
        depth_max: f32,
    },
    SetScissor {
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    },
    Draw {
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        base_vertex: i32,
        first_instance: u32,
    },
    MultiDrawIndirect {
        buffer_id: id::BufferId,
        offset: BufferAddress,
        /// Count of `None` represents a non-multi call.
        count: Option<u32>,
        indexed: bool,
    },
    MultiDrawIndirectCount {
        buffer_id: id::BufferId,
        offset: BufferAddress,
        count_buffer_id: id::BufferId,
        count_buffer_offset: BufferAddress,
        max_count: u32,
        indexed: bool,
    },
    PushDebugGroup {
        color: u32,
        len: usize,
    },
    PopDebugGroup,
    InsertDebugMarker {
        color: u32,
        len: usize,
    },
    WriteTimestamp {
        query_set_id: id::QuerySetId,
        query_index: u32,
    },
    BeginOcclusionQuery {
        query_index: u32,
    },
    EndOcclusionQuery,
    BeginPipelineStatisticsQuery {
        query_set_id: id::QuerySetId,
        query_index: u32,
    },
    EndPipelineStatisticsQuery,
    ExecuteBundle(id::RenderBundleId),
}

#[no_mangle]
pub unsafe extern "C" fn wgpu_recorded_render_pass_set_bind_group(
    pass: &mut RecordedRenderPass,
    index: u32,
    bind_group_id: Option<id::BindGroupId>,
    offsets: FfiSlice<'_, DynamicOffset>,
) {
    let offsets = offsets.as_slice();
    pass.base.dynamic_offsets.extend_from_slice(offsets);

    pass.base.commands.push(RenderCommand::SetBindGroup {
        index,
        num_dynamic_offsets: offsets.len(),
        bind_group_id,
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_set_pipeline(
    pass: &mut RecordedRenderPass,
    pipeline_id: id::RenderPipelineId,
) {
    pass.base
        .commands
        .push(RenderCommand::SetPipeline(pipeline_id));
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_set_vertex_buffer(
    pass: &mut RecordedRenderPass,
    slot: u32,
    buffer_id: Option<id::BufferId>,
    offset: BufferAddress,
    size: Option<&BufferSize>,
) {
    pass.base.commands.push(RenderCommand::SetVertexBuffer {
        slot,
        buffer_id,
        offset,
        size: size.copied(),
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_set_index_buffer(
    pass: &mut RecordedRenderPass,
    buffer_id: id::BufferId,
    index_format: IndexFormat,
    offset: BufferAddress,
    size: Option<&BufferSize>,
) {
    pass.base.commands.push(RenderCommand::SetIndexBuffer {
        buffer_id,
        index_format,
        offset,
        size: size.copied(),
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_set_blend_constant(
    pass: &mut RecordedRenderPass,
    color: &Color,
) {
    pass.base
        .commands
        .push(RenderCommand::SetBlendConstant(*color));
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_set_stencil_reference(
    pass: &mut RecordedRenderPass,
    value: u32,
) {
    pass.base
        .commands
        .push(RenderCommand::SetStencilReference(value));
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_set_viewport(
    pass: &mut RecordedRenderPass,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    depth_min: f32,
    depth_max: f32,
) {
    pass.base.commands.push(RenderCommand::SetViewport {
        x,
        y,
        w,
        h,
        depth_min,
        depth_max,
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_set_scissor_rect(
    pass: &mut RecordedRenderPass,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) {
    pass.base
        .commands
        .push(RenderCommand::SetScissor { x, y, w, h });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_draw(
    pass: &mut RecordedRenderPass,
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
) {
    pass.base.commands.push(RenderCommand::Draw {
        vertex_count,
        instance_count,
        first_vertex,
        first_instance,
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_draw_indexed(
    pass: &mut RecordedRenderPass,
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
) {
    pass.base.commands.push(RenderCommand::DrawIndexed {
        index_count,
        instance_count,
        first_index,
        base_vertex,
        first_instance,
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_draw_indirect(
    pass: &mut RecordedRenderPass,
    buffer_id: id::BufferId,
    offset: BufferAddress,
) {
    pass.base.commands.push(RenderCommand::MultiDrawIndirect {
        buffer_id,
        offset,
        count: None,
        indexed: false,
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_draw_indexed_indirect(
    pass: &mut RecordedRenderPass,
    buffer_id: id::BufferId,
    offset: BufferAddress,
) {
    pass.base.commands.push(RenderCommand::MultiDrawIndirect {
        buffer_id,
        offset,
        count: None,
        indexed: true,
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_multi_draw_indirect(
    pass: &mut RecordedRenderPass,
    buffer_id: id::BufferId,
    offset: BufferAddress,
    count: u32,
) {
    pass.base.commands.push(RenderCommand::MultiDrawIndirect {
        buffer_id,
        offset,
        count: Some(count),
        indexed: false,
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_multi_draw_indexed_indirect(
    pass: &mut RecordedRenderPass,
    buffer_id: id::BufferId,
    offset: BufferAddress,
    count: u32,
) {
    pass.base.commands.push(RenderCommand::MultiDrawIndirect {
        buffer_id,
        offset,
        count: Some(count),
        indexed: true,
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_multi_draw_indirect_count(
    pass: &mut RecordedRenderPass,
    buffer_id: id::BufferId,
    offset: BufferAddress,
    count_buffer_id: id::BufferId,
    count_buffer_offset: BufferAddress,
    max_count: u32,
) {
    pass.base
        .commands
        .push(RenderCommand::MultiDrawIndirectCount {
            buffer_id,
            offset,
            count_buffer_id,
            count_buffer_offset,
            max_count,
            indexed: false,
        });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_multi_draw_indexed_indirect_count(
    pass: &mut RecordedRenderPass,
    buffer_id: id::BufferId,
    offset: BufferAddress,
    count_buffer_id: id::BufferId,
    count_buffer_offset: BufferAddress,
    max_count: u32,
) {
    pass.base
        .commands
        .push(RenderCommand::MultiDrawIndirectCount {
            buffer_id,
            offset,
            count_buffer_id,
            count_buffer_offset,
            max_count,
            indexed: true,
        });
}

/// # Safety
///
/// This function is unsafe as there is no guarantee that the given `label`
/// is a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn wgpu_recorded_render_pass_push_debug_group(
    pass: &mut RecordedRenderPass,
    label: RawString,
    color: u32,
) {
    let bytes = unsafe { ffi::CStr::from_ptr(label) }.to_bytes();
    pass.base.string_data.extend_from_slice(bytes);

    pass.base.commands.push(RenderCommand::PushDebugGroup {
        color,
        len: bytes.len(),
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_pop_debug_group(pass: &mut RecordedRenderPass) {
    pass.base.commands.push(RenderCommand::PopDebugGroup);
}

/// # Safety
///
/// This function is unsafe as there is no guarantee that the given `label`
/// is a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn wgpu_recorded_render_pass_insert_debug_marker(
    pass: &mut RecordedRenderPass,
    label: RawString,
    color: u32,
) {
    let bytes = unsafe { ffi::CStr::from_ptr(label) }.to_bytes();
    pass.base.string_data.extend_from_slice(bytes);

    pass.base.commands.push(RenderCommand::InsertDebugMarker {
        color,
        len: bytes.len(),
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_write_timestamp(
    pass: &mut RecordedRenderPass,
    query_set_id: id::QuerySetId,
    query_index: u32,
) {
    pass.base.commands.push(RenderCommand::WriteTimestamp {
        query_set_id,
        query_index,
    });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_begin_occlusion_query(
    pass: &mut RecordedRenderPass,
    query_index: u32,
) {
    pass.base
        .commands
        .push(RenderCommand::BeginOcclusionQuery { query_index });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_end_occlusion_query(pass: &mut RecordedRenderPass) {
    pass.base.commands.push(RenderCommand::EndOcclusionQuery);
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_begin_pipeline_statistics_query(
    pass: &mut RecordedRenderPass,
    query_set_id: id::QuerySetId,
    query_index: u32,
) {
    pass.base
        .commands
        .push(RenderCommand::BeginPipelineStatisticsQuery {
            query_set_id,
            query_index,
        });
}

#[no_mangle]
pub extern "C" fn wgpu_recorded_render_pass_end_pipeline_statistics_query(
    pass: &mut RecordedRenderPass,
) {
    pass.base
        .commands
        .push(RenderCommand::EndPipelineStatisticsQuery);
}

#[no_mangle]
pub unsafe extern "C" fn wgpu_recorded_render_pass_execute_bundles(
    pass: &mut RecordedRenderPass,
    render_bundles: FfiSlice<'_, id::RenderBundleId>,
) {
    for &bundle_id in render_bundles.as_slice() {
        pass.base
            .commands
            .push(RenderCommand::ExecuteBundle(bundle_id));
    }
}

#[no_mangle]
pub unsafe extern "C" fn wgpu_client_compute_pass_encoder_set_bind_group(
    client: &Client,
    device_id: id::DeviceId,
    encoder_id: id::ComputePassEncoderId,
    index: u32,
    bind_group: Option<id::BindGroupId>,
    dynamic_offsets: FfiSlice<'_, DynamicOffset>,
) {
    let command = ComputePassEncoderCommand::BindingCommand(BindingCommand::SetBindGroup {
        index,
        bind_group,
        dynamic_offsets: dynamic_offsets.as_slice().to_vec(),
    });
    let message = Message::ComputePassEncoder(device_id, encoder_id, command);
    client.queue_message(&message);
}

#[no_mangle]
pub extern "C" fn wgpu_client_compute_pass_encoder_set_pipeline(
    client: &Client,
    device_id: id::DeviceId,
    encoder_id: id::ComputePassEncoderId,
    pipeline_id: id::ComputePipelineId,
) {
    let command = ComputePassEncoderCommand::SetPipeline(pipeline_id);
    let message = Message::ComputePassEncoder(device_id, encoder_id, command);
    client.queue_message(&message);
}

#[no_mangle]
pub extern "C" fn wgpu_client_compute_pass_encoder_dispatch_workgroups(
    client: &Client,
    device_id: id::DeviceId,
    encoder_id: id::ComputePassEncoderId,
    workgroup_count_x: u32,
    workgroup_count_y: u32,
    workgroup_count_z: u32,
) {
    let command = ComputePassEncoderCommand::DispatchWorkgroups {
        workgroup_count_x,
        workgroup_count_y,
        workgroup_count_z,
    };
    let message = Message::ComputePassEncoder(device_id, encoder_id, command);
    client.queue_message(&message);
}

#[no_mangle]
pub extern "C" fn wgpu_client_compute_pass_encoder_dispatch_workgroups_indirect(
    client: &Client,
    device_id: id::DeviceId,
    encoder_id: id::ComputePassEncoderId,
    indirect_buffer: id::BufferId,
    indirect_offset: BufferAddress,
) {
    let command = ComputePassEncoderCommand::DispatchWorkgroupsIndirect {
        indirect_buffer,
        indirect_offset,
    };
    let message = Message::ComputePassEncoder(device_id, encoder_id, command);
    client.queue_message(&message);
}

/// # Safety
///
/// This function is unsafe as there is no guarantee that the given `label`
/// is a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn wgpu_client_compute_pass_encoder_push_debug_group(
    client: &Client,
    device_id: id::DeviceId,
    encoder_id: id::ComputePassEncoderId,
    label: RawString,
) {
    let command = ComputePassEncoderCommand::DebugCommand(DebugCommand::PushDebugGroup(
        raw_string_to_string(label),
    ));
    let message = Message::ComputePassEncoder(device_id, encoder_id, command);
    client.queue_message(&message);
}

#[no_mangle]
pub extern "C" fn wgpu_client_compute_pass_encoder_pop_debug_group(
    client: &Client,
    device_id: id::DeviceId,
    encoder_id: id::ComputePassEncoderId,
) {
    let command = ComputePassEncoderCommand::DebugCommand(DebugCommand::PopDebugGroup);
    let message = Message::ComputePassEncoder(device_id, encoder_id, command);
    client.queue_message(&message);
}

/// # Safety
///
/// This function is unsafe as there is no guarantee that the given `label`
/// is a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn wgpu_client_compute_pass_encoder_insert_debug_marker(
    client: &Client,
    device_id: id::DeviceId,
    encoder_id: id::ComputePassEncoderId,
    label: RawString,
) {
    let command = ComputePassEncoderCommand::DebugCommand(DebugCommand::InsertDebugMarker(
        raw_string_to_string(label),
    ));
    let message = Message::ComputePassEncoder(device_id, encoder_id, command);
    client.queue_message(&message);
}

#[no_mangle]
pub unsafe extern "C" fn wgpu_client_compute_pass_encoder_end(
    client: &Client,
    device_id: id::DeviceId,
    encoder_id: id::ComputePassEncoderId,
) {
    let command = ComputePassEncoderCommand::End;
    let message = Message::ComputePassEncoder(device_id, encoder_id, command);
    client.queue_message(&message);
}

pub fn replay_render_pass(
    global: &Global,
    device_id: id::DeviceId,
    id: CommandEncoderId,
    src_pass: &RecordedRenderPass,
    error_buf: &mut crate::error::OwnedErrorBuffer,
) {
    let (mut dst_pass, err) = global.command_encoder_begin_render_pass(
        id,
        &wgc::command::RenderPassDescriptor {
            label: src_pass.base.label.as_ref().map(|s| s.as_str().into()),
            color_attachments: Cow::Borrowed(&src_pass.color_attachments),
            depth_stencil_attachment: src_pass.depth_stencil_attachment.clone(),
            timestamp_writes: src_pass.timestamp_writes.clone(),
            occlusion_query_set: src_pass.occlusion_query_set_id,
            multiview_mask: None,
        },
    );
    if let Some(err) = err {
        error_buf.init(err, device_id);
        return;
    }
    match replay_render_pass_impl(global, src_pass, &mut dst_pass) {
        Ok(()) => (),
        Err(err) => {
            error_buf.init(err, device_id);
            return;
        }
    };

    match global.render_pass_end(&mut dst_pass) {
        Ok(()) => (),
        Err(err) => error_buf.init(err, device_id),
    }
}

pub fn replay_render_pass_impl(
    global: &Global,
    src_pass: &RecordedRenderPass,
    dst_pass: &mut wgc::command::RenderPass,
) -> Result<(), wgc::command::PassStateError> {
    let mut dynamic_offsets = src_pass.base.dynamic_offsets.as_slice();
    let mut dynamic_offsets = |len| {
        let offsets;
        (offsets, dynamic_offsets) = dynamic_offsets.split_at(len);
        offsets
    };
    let mut strings = src_pass.base.string_data.as_slice();
    let mut strings = |len| {
        let label;
        (label, strings) = strings.split_at(len);
        label
    };
    for command in &src_pass.base.commands {
        match *command {
            RenderCommand::SetBindGroup {
                index,
                num_dynamic_offsets,
                bind_group_id,
            } => {
                let offsets = dynamic_offsets(num_dynamic_offsets);
                global.render_pass_set_bind_group(dst_pass, index, bind_group_id, offsets)
            }
            RenderCommand::SetPipeline(pipeline_id) => {
                global.render_pass_set_pipeline(dst_pass, pipeline_id)
            }
            RenderCommand::SetIndexBuffer {
                buffer_id,
                index_format,
                offset,
                size,
            } => {
                global.render_pass_set_index_buffer(dst_pass, buffer_id, index_format, offset, size)
            }
            RenderCommand::SetVertexBuffer {
                slot,
                buffer_id,
                offset,
                size,
            } => global.render_pass_set_vertex_buffer(dst_pass, slot, buffer_id, offset, size),
            RenderCommand::SetBlendConstant(color) => {
                global.render_pass_set_blend_constant(dst_pass, color)
            }
            RenderCommand::SetStencilReference(value) => {
                global.render_pass_set_stencil_reference(dst_pass, value)
            }
            RenderCommand::SetViewport {
                x,
                y,
                w,
                h,
                depth_min,
                depth_max,
            } => global.render_pass_set_viewport(dst_pass, x, y, w, h, depth_min, depth_max),
            RenderCommand::SetScissor { x, y, w, h } => {
                global.render_pass_set_scissor_rect(dst_pass, x, y, w, h)
            }
            RenderCommand::Draw {
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            } => global.render_pass_draw(
                dst_pass,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            ),
            RenderCommand::DrawIndexed {
                index_count,
                instance_count,
                first_index,
                base_vertex,
                first_instance,
            } => global.render_pass_draw_indexed(
                dst_pass,
                index_count,
                instance_count,
                first_index,
                base_vertex,
                first_instance,
            ),
            RenderCommand::MultiDrawIndirect {
                buffer_id,
                offset,
                count,
                indexed,
            } => match (indexed, count) {
                (false, Some(count)) => {
                    global.render_pass_multi_draw_indirect(dst_pass, buffer_id, offset, count)
                }
                (false, None) => global.render_pass_draw_indirect(dst_pass, buffer_id, offset),
                (true, Some(count)) => global
                    .render_pass_multi_draw_indexed_indirect(dst_pass, buffer_id, offset, count),
                (true, None) => {
                    global.render_pass_draw_indexed_indirect(dst_pass, buffer_id, offset)
                }
            },
            RenderCommand::MultiDrawIndirectCount {
                buffer_id,
                offset,
                count_buffer_id,
                count_buffer_offset,
                max_count,
                indexed,
            } => {
                if indexed {
                    global.render_pass_multi_draw_indexed_indirect_count(
                        dst_pass,
                        buffer_id,
                        offset,
                        count_buffer_id,
                        count_buffer_offset,
                        max_count,
                    )
                } else {
                    global.render_pass_multi_draw_indirect_count(
                        dst_pass,
                        buffer_id,
                        offset,
                        count_buffer_id,
                        count_buffer_offset,
                        max_count,
                    )
                }
            }
            RenderCommand::PushDebugGroup { color, len } => {
                let label = strings(len);
                let label = std::str::from_utf8(label).unwrap();
                global.render_pass_push_debug_group(dst_pass, label, color)
            }
            RenderCommand::PopDebugGroup => global.render_pass_pop_debug_group(dst_pass),
            RenderCommand::InsertDebugMarker { color, len } => {
                let label = strings(len);
                let label = std::str::from_utf8(label).unwrap();
                global.render_pass_insert_debug_marker(dst_pass, label, color)
            }
            RenderCommand::WriteTimestamp {
                query_set_id,
                query_index,
            } => global.render_pass_write_timestamp(dst_pass, query_set_id, query_index),
            RenderCommand::BeginOcclusionQuery { query_index } => {
                global.render_pass_begin_occlusion_query(dst_pass, query_index)
            }
            RenderCommand::EndOcclusionQuery => global.render_pass_end_occlusion_query(dst_pass),
            RenderCommand::BeginPipelineStatisticsQuery {
                query_set_id,
                query_index,
            } => global.render_pass_begin_pipeline_statistics_query(
                dst_pass,
                query_set_id,
                query_index,
            ),
            RenderCommand::EndPipelineStatisticsQuery => {
                global.render_pass_end_pipeline_statistics_query(dst_pass)
            }
            RenderCommand::ExecuteBundle(bundle_id) => {
                global.render_pass_execute_bundles(dst_pass, &[bundle_id])
            }
        }?
    }

    Ok(())
}

pub(crate) fn compute_pass_encoder_command(
    global: &Global,
    device_id: id::DeviceId,
    id: id::ComputePassEncoderId,
    cmd: ComputePassEncoderCommand,
    error_buf: &mut crate::error::OwnedErrorBuffer,
) {
    let res = match cmd {
        ComputePassEncoderCommand::BindingCommand(binding_command) => match binding_command {
            BindingCommand::SetBindGroup {
                index,
                bind_group,
                dynamic_offsets,
            } => {
                global.compute_pass_set_bind_group_with_id(id, index, bind_group, &dynamic_offsets)
            }
            BindingCommand::SetImmediates { range_offset, data } => {
                global.compute_pass_set_immediates_with_id(id, range_offset, &data)
            }
        },
        ComputePassEncoderCommand::SetPipeline(pipeline_id) => {
            global.compute_pass_set_pipeline_with_id(id, pipeline_id)
        }
        ComputePassEncoderCommand::DispatchWorkgroups {
            workgroup_count_x,
            workgroup_count_y,
            workgroup_count_z,
        } => global.compute_pass_dispatch_workgroups_with_id(
            id,
            workgroup_count_x,
            workgroup_count_y,
            workgroup_count_z,
        ),
        ComputePassEncoderCommand::DispatchWorkgroupsIndirect {
            indirect_buffer,
            indirect_offset,
        } => global.compute_pass_dispatch_workgroups_indirect_with_id(
            id,
            indirect_buffer,
            indirect_offset,
        ),
        ComputePassEncoderCommand::DebugCommand(debug_command) => match debug_command {
            DebugCommand::PushDebugGroup(label) => {
                global.compute_pass_push_debug_group_with_id(id, &label, 0)
            }
            DebugCommand::PopDebugGroup => global.compute_pass_pop_debug_group_with_id(id),
            DebugCommand::InsertDebugMarker(label) => {
                global.compute_pass_insert_debug_marker_with_id(id, &label, 0)
            }
        },
        ComputePassEncoderCommand::End => {
            let res = global.compute_pass_end_with_id(id);
            if let Err(err) = res {
                error_buf.init(err, device_id);
            }
            Ok(())
        }
    };
    if let Err(err) = res {
        error_buf.init(err, device_id);
    }
}
