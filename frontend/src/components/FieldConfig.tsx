import {
  Button,
  Checkbox,
  Input,
  InputNumber,
  Switch,
  Table,
  Tag,
  Typography,
} from "@douyinfe/semi-ui";
import { useMemo, useState } from "react";
import type { ColumnInfo, NumberFormat } from "../types";
import { BITABLE_TYPE_LABELS } from "../types";

const { Text } = Typography;

interface FieldConfigProps {
  columns: ColumnInfo[];
  selectedFields: string[] | null;
  onSelectedFieldsChange: (fields: string[] | null) => void;
  fieldRenames: Record<string, string>;
  onFieldRenamesChange: (renames: Record<string, string>) => void;
  numberFormats: Record<string, NumberFormat>;
  onNumberFormatsChange: (formats: Record<string, NumberFormat>) => void;
  onNext: () => void;
  onBack: () => void;
}

export function FieldConfig({
  columns,
  selectedFields,
  onSelectedFieldsChange,
  fieldRenames,
  onFieldRenamesChange,
  numberFormats,
  onNumberFormatsChange,
  onNext,
  onBack,
}: FieldConfigProps) {
  const [selectAll, setSelectAll] = useState(selectedFields === null);

  const isSelected = (name: string) =>
    selectedFields === null || selectedFields.includes(name);

  const toggleField = (name: string) => {
    if (selectedFields === null) {
      const newSelected = columns
        .map((c) => c.name)
        .filter((n) => n !== name);
      onSelectedFieldsChange(newSelected);
      setSelectAll(false);
    } else if (selectedFields.includes(name)) {
      onSelectedFieldsChange(selectedFields.filter((f) => f !== name));
    } else {
      onSelectedFieldsChange([...selectedFields, name]);
    }
  };

  const handleSelectAll = (checked: boolean) => {
    setSelectAll(checked);
    onSelectedFieldsChange(checked ? null : columns.map((c) => c.name));
  };

  const updateRename = (colName: string, displayName: string) => {
    if (displayName === colName || !displayName) {
      const next = { ...fieldRenames };
      delete next[colName];
      onFieldRenamesChange(next);
    } else {
      onFieldRenamesChange({ ...fieldRenames, [colName]: displayName });
    }
  };

  const updateNumberFormat = (colName: string, precision: number) => {
    if (precision === 0) {
      const next = { ...numberFormats };
      delete next[colName];
      onNumberFormatsChange(next);
    } else {
      onNumberFormatsChange({
        ...numberFormats,
        [colName]: { precision },
      });
    }
  };

  const tableColumns = useMemo(
    () => [
      {
        title: (
          <Checkbox
            checked={selectAll}
            onChange={(e) => handleSelectAll(e.target.checked ?? false)}
          />
        ),
        dataIndex: "select",
        key: "select",
        width: 44,
        render: (_: unknown, record: ColumnInfo) => (
          <Checkbox
            checked={isSelected(record.name)}
            onChange={() => toggleField(record.name)}
          />
        ),
      },
      {
        title: "列名",
        dataIndex: "name",
        key: "name",
        width: 110,
        render: (name: string) => (
          <Text
            strong
            style={{
              fontSize: 13,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              display: "block",
            }}
            title={name}
          >
            {name}
          </Text>
        ),
      },
      {
        title: "原始类型",
        dataIndex: "udt_name",
        key: "udt_name",
        width: 90,
        render: (_: unknown, record: ColumnInfo) => (
          <Tag
            size="small"
            color="blue"
            style={{
              fontSize: 11,
              maxWidth: "100%",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              display: "inline-block",
            }}
          >
            {record.udt_name || record.data_type}
          </Tag>
        ),
      },
      {
        title: "多维表格类型",
        dataIndex: "bitable_type",
        key: "bitable_type",
        width: 70,
        render: (v: number) => (
          <span style={{ fontSize: 13 }}>
            {BITABLE_TYPE_LABELS[v] ?? "文本"}
          </span>
        ),
      },
      {
        title: "显示名",
        dataIndex: "display_name",
        key: "display_name",
        width: 130,
        render: (_: unknown, record: ColumnInfo) => (
          <Input
            size="small"
            value={fieldRenames[record.name] ?? record.name}
            onChange={(v) => updateRename(record.name, v)}
            style={{ width: "100%" }}
          />
        ),
      },
      {
        title: "小数位",
        dataIndex: "precision",
        key: "precision",
        width: 70,
        render: (_: unknown, record: ColumnInfo) => {
          if (record.bitable_type !== 2 && record.bitable_type !== 8) {
            return <Text type="quaternary">-</Text>;
          }
          return (
            <InputNumber
              size="small"
              value={numberFormats[record.name]?.precision ?? 0}
              onChange={(v) =>
                updateNumberFormat(record.name, (v as number) ?? 0)
              }
              min={0}
              max={10}
              style={{ width: 56 }}
            />
          );
        },
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [columns, selectedFields, fieldRenames, numberFormats, selectAll],
  );

  const selectedCount =
    selectedFields === null ? columns.length : selectedFields.length;

  return (
    <div>
      <div
        style={{
          marginBottom: 10,
          display: "flex",
          alignItems: "center",
          gap: 8,
        }}
      >
        <Switch
          checked={selectAll}
          onChange={handleSelectAll}
          size="small"
        />
        <Text style={{ fontSize: 13 }}>
          {"全部字段（含未来新增）"}
        </Text>
        <Text
          type="tertiary"
          style={{ fontSize: 12, marginLeft: "auto" }}
        >
          {`已选 ${selectedCount}/${columns.length}`}
        </Text>
      </div>

      <Table
        columns={tableColumns}
        dataSource={columns}
        rowKey="name"
        size="small"
        pagination={false}
        scroll={{ y: 280 }}
        style={{ tableLayout: "fixed" }}
      />

      <div className="footer-actions">
        <Button onClick={onBack}>{"上一步"}</Button>
        <Button
          theme="solid"
          onClick={onNext}
          disabled={selectedCount === 0}
        >
          {"下一步"}
        </Button>
      </div>
    </div>
  );
}
