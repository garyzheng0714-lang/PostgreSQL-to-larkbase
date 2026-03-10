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
          >
            All
          </Checkbox>
        ),
        dataIndex: "select",
        key: "select",
        width: 70,
        render: (_: unknown, record: ColumnInfo) => (
          <Checkbox
            checked={isSelected(record.name)}
            onChange={() => toggleField(record.name)}
          />
        ),
      },
      {
        title: "Column",
        dataIndex: "name",
        key: "name",
        width: 140,
        render: (name: string) => <Text strong>{name}</Text>,
      },
      {
        title: "PG Type",
        dataIndex: "data_type",
        key: "data_type",
        width: 120,
        render: (v: string) => <Tag size="small">{v}</Tag>,
      },
      {
        title: "Bitable Type",
        dataIndex: "bitable_type",
        key: "bitable_type",
        width: 100,
        render: (v: number) => BITABLE_TYPE_LABELS[v] ?? "Text",
      },
      {
        title: "Display Name",
        dataIndex: "display_name",
        key: "display_name",
        width: 160,
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
        title: "Decimals",
        dataIndex: "precision",
        key: "precision",
        width: 90,
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
              style={{ width: 70 }}
            />
          );
        },
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [columns, selectedFields, fieldRenames, numberFormats, selectAll]
  );

  const selectedCount =
    selectedFields === null ? columns.length : selectedFields.length;

  return (
    <div>
      <div style={{ marginBottom: 12 }}>
        <Switch
          checked={selectAll}
          onChange={handleSelectAll}
          style={{ marginRight: 8 }}
        />
        <Text>Include all fields (including future additions)</Text>
        <Text type="tertiary" style={{ marginLeft: 12 }}>
          {selectedCount} / {columns.length} fields selected
        </Text>
      </div>

      <Table
        columns={tableColumns}
        dataSource={columns}
        rowKey="name"
        size="small"
        pagination={false}
        scroll={{ y: 320 }}
      />

      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          marginTop: 24,
        }}
      >
        <Button onClick={onBack}>Back</Button>
        <Button
          theme="solid"
          onClick={onNext}
          disabled={selectedCount === 0}
        >
          Next
        </Button>
      </div>
    </div>
  );
}
