import { useEffect, useState, type FormEvent } from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Label,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@ora/ui";
import { useTranslation } from "react-i18next";

export interface EntityField {
  name: string;
  label: string;
  value: string;
  placeholder?: string;
  options?: Array<{ label: string; value: string }>;
}

interface EntityDialogProps {
  open: boolean;
  title: string;
  description: string;
  submitLabel: string;
  fields: EntityField[];
  onOpenChange: (open: boolean) => void;
  onSubmit: (values: Record<string, string>) => Promise<void>;
}

/** Provides one consistent create/edit form for every level of the workspace tree. */
export function EntityDialog({
  open,
  title,
  description,
  submitLabel,
  fields,
  onOpenChange,
  onSubmit,
}: EntityDialogProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<Record<string, string>>({});
  const [submitting, setSubmitting] = useState(false);
  const [validationError, setValidationError] = useState(false);

  useEffect(() => {
    if (open) {
      setValues(Object.fromEntries(fields.map((field) => [field.name, field.value])));
      setValidationError(false);
    }
  }, [fields, open]);

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (fields.some((field) => !values[field.name]?.trim())) {
      setValidationError(true);
      return;
    }
    setSubmitting(true);
    try {
      await onSubmit(values);
      onOpenChange(false);
    } catch {
      // The workspace surfaces transport errors inline, so the form stays open for correction or retry.
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => (!submitting || nextOpen) && onOpenChange(nextOpen)}>
      <DialogContent>
        <form onSubmit={handleSubmit} className="contents">
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            <DialogDescription>{description}</DialogDescription>
          </DialogHeader>
          <div className="grid gap-3">
            {fields.map((field) => (
              <div key={field.name} className="grid gap-1.5">
                <Label htmlFor={`entity-${field.name}`}>{field.label}</Label>
                {field.options ? (
                  <Select
                    value={values[field.name] ?? ""}
                    onValueChange={(value) => setValues((current) => ({ ...current, [field.name]: value ?? "" }))}
                  >
                    <SelectTrigger id={`entity-${field.name}`} className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {field.options.map((option) => (
                        <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                ) : (
                  <Input
                    id={`entity-${field.name}`}
                    value={values[field.name] ?? ""}
                    placeholder={field.placeholder}
                    aria-invalid={validationError && !values[field.name]?.trim()}
                    onChange={(event) => {
                      setValues((current) => ({ ...current, [field.name]: event.target.value }));
                      setValidationError(false);
                    }}
                    autoFocus={field === fields[0]}
                  />
                )}
              </div>
            ))}
          </div>
          {validationError && <p role="alert" className="text-xs text-destructive">{t("dialog.required")}</p>}
          <DialogFooter>
            <Button type="button" variant="outline" disabled={submitting} onClick={() => onOpenChange(false)}>{t("common.cancel")}</Button>
            <Button type="submit" disabled={submitting}>{submitting ? t("common.saving") : submitLabel}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
