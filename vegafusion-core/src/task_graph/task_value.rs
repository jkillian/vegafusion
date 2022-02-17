/*
 * VegaFusion
 * Copyright (C) 2022 Jon Mease
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public
 * License along with this program.
 * If not, see http://www.gnu.org/licenses/.
 */
use crate::data::scalar::{ScalarValue, ScalarValueHelpers};
use crate::data::table::VegaFusionTable;
use crate::error::{Result, VegaFusionError};
use crate::proto::gen::tasks::task_value::Data;
use crate::proto::gen::tasks::TaskValue as ProtoTaskValue;
use crate::task_graph::memory::{inner_size_of_scalar, inner_size_of_table};
use arrow::record_batch::RecordBatch;
use serde_json::Value;
use std::convert::TryFrom;

#[derive(Debug, Clone)]
pub enum TaskValue {
    Scalar(ScalarValue),
    Table(VegaFusionTable),
}

impl TaskValue {
    pub fn as_scalar(&self) -> Result<&ScalarValue> {
        match self {
            TaskValue::Scalar(value) => Ok(value),
            _ => Err(VegaFusionError::internal("Value is not a scalar")),
        }
    }

    pub fn as_table(&self) -> Result<&VegaFusionTable> {
        match self {
            TaskValue::Table(value) => Ok(value),
            _ => Err(VegaFusionError::internal("Value is not a table")),
        }
    }

    pub fn to_json(&self) -> Result<Value> {
        match self {
            TaskValue::Scalar(value) => value.to_json(),
            TaskValue::Table(value) => Ok(value.to_json()),
        }
    }

    pub fn size_of(&self) -> usize {
        let inner_size = match self {
            TaskValue::Scalar(scalar) => inner_size_of_scalar(scalar),
            TaskValue::Table(table) => inner_size_of_table(table),
        };

        std::mem::size_of::<Self>() + inner_size
    }
}

impl TryFrom<&ProtoTaskValue> for TaskValue {
    type Error = VegaFusionError;

    fn try_from(value: &ProtoTaskValue) -> std::result::Result<Self, Self::Error> {
        match value.data.as_ref().unwrap() {
            Data::Table(value) => Ok(Self::Table(VegaFusionTable::from_ipc_bytes(value)?)),
            Data::Scalar(value) => {
                let scalar_table = VegaFusionTable::from_ipc_bytes(value)?;
                let scalar_rb = scalar_table.to_record_batch()?;
                let scalar_array = scalar_rb.column(0);
                let scalar = ScalarValue::try_from_array(scalar_array, 0)?;
                Ok(Self::Scalar(scalar))
            }
        }
    }
}

impl TryFrom<&TaskValue> for ProtoTaskValue {
    type Error = VegaFusionError;

    fn try_from(value: &TaskValue) -> std::result::Result<Self, Self::Error> {
        match value {
            TaskValue::Scalar(scalar) => {
                let scalar_array = scalar.to_array();
                let scalar_rb = RecordBatch::try_from_iter(vec![("value", scalar_array)])?;
                let ipc_bytes = VegaFusionTable::from(scalar_rb).to_ipc_bytes()?;
                Ok(Self {
                    data: Some(Data::Scalar(ipc_bytes)),
                })
            }
            TaskValue::Table(table) => Ok(Self {
                data: Some(Data::Table(table.to_ipc_bytes()?)),
            }),
        }
    }
}
