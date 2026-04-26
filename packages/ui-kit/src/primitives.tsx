import {
  Alert as HeroAlert,
  Button as HeroButton,
  ButtonGroup as HeroButtonGroup,
  Card as HeroCard,
  Checkbox as HeroCheckbox,
  Chip as HeroChip,
  Description as HeroDescription,
  FieldError as HeroFieldError,
  Input as HeroInput,
  Label as HeroLabel,
  Link as HeroLink,
  NumberField as HeroNumberField,
  NumberFieldDecrementButton as HeroNumberFieldDecrementButton,
  NumberFieldGroup as HeroNumberFieldGroup,
  NumberFieldIncrementButton as HeroNumberFieldIncrementButton,
  NumberFieldInput as HeroNumberFieldInput,
  Separator as HeroSeparator,
  Spinner as HeroSpinner,
  Switch as HeroSwitch,
  TextArea as HeroTextArea,
  Toast,
} from "@heroui/react";
import type {
  CSSProperties,
  ChangeEvent,
  ComponentPropsWithoutRef,
  ElementType,
  PropsWithChildren,
  ReactNode,
} from "react";

const HeroAlertRoot = HeroAlert as ElementType;
const HeroButtonRoot = HeroButton as ElementType;
const HeroButtonGroupRoot = HeroButtonGroup as ElementType;
const HeroCardRoot = HeroCard as ElementType;
const HeroCheckboxRoot = HeroCheckbox as ElementType;
const HeroChipRoot = HeroChip as ElementType;
const HeroDescriptionRoot = HeroDescription as ElementType;
const HeroFieldErrorRoot = HeroFieldError as ElementType;
const HeroInputRoot = HeroInput as ElementType;
const HeroLabelRoot = HeroLabel as ElementType;
const HeroLinkRoot = HeroLink as ElementType;
const HeroNumberFieldDecrementButtonRoot =
  HeroNumberFieldDecrementButton as ElementType;
const HeroNumberFieldGroupRoot = HeroNumberFieldGroup as ElementType;
const HeroNumberFieldInputRoot = HeroNumberFieldInput as ElementType;
const HeroNumberFieldIncrementButtonRoot =
  HeroNumberFieldIncrementButton as ElementType;
const HeroNumberFieldRoot = HeroNumberField as ElementType;
const HeroSeparatorRoot = HeroSeparator as ElementType;
const HeroSpinnerRoot = HeroSpinner as ElementType;
const HeroSwitchRoot = HeroSwitch as ElementType;
const HeroTextAreaRoot = HeroTextArea as ElementType;

type Spacing = number | string;
type Size = number | string;
type Tone = string;
type Variant = string;

type LayoutProps = {
  align?: CSSProperties["alignItems"];
  bg?: CSSProperties["background"];
  c?: string;
  className?: string;
  gap?: Spacing;
  grow?: boolean | number;
  h?: CSSProperties["height"];
  justify?: CSSProperties["justifyContent"];
  m?: Spacing;
  maw?: CSSProperties["maxWidth"];
  mb?: Spacing;
  mih?: CSSProperties["minHeight"];
  ml?: Spacing;
  mr?: Spacing;
  mt?: Spacing;
  mx?: Spacing;
  my?: Spacing;
  p?: Spacing;
  pb?: Spacing;
  pl?: Spacing;
  pr?: Spacing;
  pt?: Spacing;
  px?: Spacing;
  py?: Spacing;
  style?: CSSProperties;
  w?: CSSProperties["width"];
};

function spacing(value: Spacing | undefined) {
  if (typeof value === "number") {
    return value;
  }

  if (value === "xs") {
    return 6;
  }

  if (value === "sm") {
    return 10;
  }

  if (value === "md") {
    return 16;
  }

  if (value === "lg") {
    return 24;
  }

  if (value === "xl") {
    return 32;
  }

  return value;
}

function color(value: string | undefined) {
  if (!value) {
    return undefined;
  }

  if (value === "dimmed") {
    return "var(--muted-foreground)";
  }

  return value;
}

function layoutStyle(props: LayoutProps): CSSProperties {
  return {
    alignItems: props.align,
    background: props.bg,
    color: color(props.c),
    flexGrow: props.grow === true ? 1 : props.grow || undefined,
    gap: spacing(props.gap),
    height: props.h,
    justifyContent: props.justify,
    margin: spacing(props.m),
    marginBlock: spacing(props.my),
    marginBlockEnd: spacing(props.mb),
    marginBlockStart: spacing(props.mt),
    marginInline: spacing(props.mx),
    marginInlineEnd: spacing(props.mr),
    marginInlineStart: spacing(props.ml),
    maxWidth: props.maw,
    minHeight: props.mih,
    padding: spacing(props.p),
    paddingBlock: spacing(props.py),
    paddingBlockEnd: spacing(props.pb),
    paddingBlockStart: spacing(props.pt),
    paddingInline: spacing(props.px),
    paddingInlineEnd: spacing(props.pr),
    paddingInlineStart: spacing(props.pl),
    width: props.w,
    ...props.style,
  };
}

function classNames(...values: Array<false | null | string | undefined>) {
  return values.filter(Boolean).join(" ");
}

export function UiProvider({ children }: PropsWithChildren) {
  return (
    <>
      {children}
      <Toast.Provider placement="bottom end" />
    </>
  );
}

type BoxProps = LayoutProps & ComponentPropsWithoutRef<"div">;

export function UiBox({ className, style, ...props }: BoxProps) {
  return (
    <div
      className={className}
      style={layoutStyle({ ...props, style })}
      {...props}
    />
  );
}

type StackProps = LayoutProps & ComponentPropsWithoutRef<"div">;

export function UiStack({ className, style, ...props }: StackProps) {
  return (
    <div
      className={classNames("hr-stack", className)}
      style={layoutStyle({ gap: "md", ...props, style })}
      {...props}
    />
  );
}

type GroupProps = LayoutProps &
  ComponentPropsWithoutRef<"div"> & {
    wrap?: CSSProperties["flexWrap"];
  };

export function UiInline({
  className,
  style,
  wrap = "wrap",
  ...props
}: GroupProps) {
  return (
    <div
      className={classNames("hr-group", className)}
      style={{ flexWrap: wrap, ...layoutStyle({ gap: "md", ...props, style }) }}
      {...props}
    />
  );
}

type SimpleGridProps = LayoutProps &
  ComponentPropsWithoutRef<"div"> & {
    cols?: number | Record<string, number>;
    spacing?: Spacing;
  };

export function UiGrid({
  className,
  cols = 1,
  spacing: gridSpacing = "md",
  style,
  ...props
}: SimpleGridProps) {
  const columnCount =
    typeof cols === "number" ? cols : (cols.lg ?? cols.sm ?? cols.base ?? 1);

  return (
    <div
      className={classNames("hr-simple-grid", className)}
      style={{
        gap: spacing(gridSpacing),
        gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
        ...layoutStyle({ ...props, style }),
      }}
      {...props}
    />
  );
}

type CenterProps = LayoutProps & ComponentPropsWithoutRef<"div">;

export function UiCenter({ className, style, ...props }: CenterProps) {
  return (
    <div
      className={classNames("hr-center", className)}
      style={layoutStyle({ ...props, style })}
      {...props}
    />
  );
}

type TextProps = LayoutProps &
  ComponentPropsWithoutRef<"p"> & {
    component?: ElementType;
    fw?: CSSProperties["fontWeight"];
    href?: string;
    size?: Size;
    to?: string;
  };

export function UiText({
  className,
  component: Component = "p",
  fw,
  size,
  style,
  ...props
}: TextProps) {
  return (
    <Component
      className={classNames("hr-text", className)}
      style={{
        fontSize: fontSize(size),
        fontWeight: fw,
        ...layoutStyle({ ...props, style }),
      }}
      {...props}
    />
  );
}

type TitleProps = LayoutProps &
  ComponentPropsWithoutRef<"h1"> & {
    order?: 1 | 2 | 3 | 4 | 5 | 6;
  };

export function UiHeading({
  className,
  order = 2,
  style,
  ...props
}: TitleProps) {
  const Heading = `h${order}` as ElementType;

  return (
    <Heading
      className={classNames("hr-title", className)}
      style={layoutStyle({ ...props, style })}
      {...props}
    />
  );
}

type CardProps = LayoutProps &
  ComponentPropsWithoutRef<"section"> & {
    padding?: Spacing;
    radius?: Size;
    shadow?: Size;
    withBorder?: boolean;
  };

export function UiSurface({
  className,
  padding = "md",
  radius,
  shadow,
  style,
  withBorder: _withBorder,
  ...props
}: CardProps) {
  return (
    <HeroCardRoot
      className={classNames(
        "hr-card",
        shadow ? "hr-card-shadow" : undefined,
        className,
      )}
      style={{
        borderRadius: radiusValue(radius),
        padding: spacing(padding),
        ...layoutStyle({ ...props, style }),
      }}
      {...props}
    />
  );
}

type BadgeProps = LayoutProps &
  ComponentPropsWithoutRef<"span"> & {
    color?: Tone;
    size?: Size;
    variant?: Variant;
  };

export function UiChip({
  className,
  color: tone,
  size,
  variant,
  style,
  ...props
}: BadgeProps) {
  return (
    <HeroChipRoot
      className={classNames(
        "hr-badge",
        toneClass(tone),
        variantClass(variant),
        sizeClass(size),
        className,
      )}
      style={layoutStyle({ ...props, style })}
      {...props}
    />
  );
}

type AlertProps = LayoutProps &
  ComponentPropsWithoutRef<"div"> & {
    color?: Tone;
    radius?: Size;
    title?: ReactNode;
    variant?: Variant;
  };

export function UiAlert({
  children,
  className,
  color: tone,
  title,
  variant,
  style,
  ...props
}: AlertProps) {
  return (
    <HeroAlertRoot
      className={classNames(
        "hr-alert",
        toneClass(tone),
        variantClass(variant),
        className,
      )}
      role="status"
      style={layoutStyle({ ...props, style })}
      {...props}
    >
      {title ? <strong>{title}</strong> : null}
      {children}
    </HeroAlertRoot>
  );
}

type ButtonProps = Omit<ComponentPropsWithoutRef<"button">, "color"> &
  LayoutProps & {
    color?: Tone;
    component?: ElementType;
    fullWidth?: boolean;
    href?: string;
    loading?: boolean;
    size?: Size;
    to?: string;
    variant?: Variant;
  };

export function UiButton({
  children,
  className,
  color: tone,
  component: Component,
  fullWidth,
  loading,
  size,
  style,
  type = "button",
  variant,
  ...props
}: ButtonProps) {
  const mergedClassName = classNames(
    "hr-button",
    fullWidth ? "hr-button-full" : undefined,
    toneClass(tone),
    variantClass(variant),
    sizeClass(size),
    className,
  );

  if (Component) {
    return (
      <Component
        className={mergedClassName}
        style={layoutStyle({ ...props, style })}
        {...props}
      >
        {loading ? <span className="hr-loader" /> : null}
        {children}
      </Component>
    );
  }

  return (
    <HeroButtonRoot
      className={mergedClassName}
      isDisabled={loading || props.disabled}
      style={layoutStyle({ ...props, style })}
      type={type}
      {...props}
    >
      {loading ? <span className="hr-loader" /> : null}
      {children}
    </HeroButtonRoot>
  );
}

type ButtonGroupProps = LayoutProps & ComponentPropsWithoutRef<"div">;

export function UiButtonGroup({
  className,
  style,
  ...props
}: ButtonGroupProps) {
  return (
    <HeroButtonGroupRoot
      className={classNames("hr-button-group", className)}
      style={layoutStyle({ ...props, style })}
      {...props}
    />
  );
}

type InputProps = Omit<ComponentPropsWithoutRef<"input">, "onChange" | "size"> &
  LayoutProps & {
    description?: ReactNode;
    error?: ReactNode;
    label?: ReactNode;
    onChange?: (event: ChangeEvent<HTMLInputElement>) => void;
    size?: Size;
  };

export function UiTextField(props: InputProps) {
  return <FieldInput type="text" {...props} />;
}

export function UiNumberField({
  className,
  description,
  error,
  label,
  onChange,
  size,
  style,
  value,
  ...props
}: Omit<InputProps, "onChange" | "value"> & {
  onChange?: (value: number | string) => void;
  value?: number | string;
}) {
  return (
    <div
      className={classNames("hr-field", className)}
      style={layoutStyle({ ...props, style })}
    >
      {label ? <HeroLabelRoot>{label}</HeroLabelRoot> : null}
      <HeroNumberFieldRoot
        onChange={(nextValue: number) => onChange?.(nextValue)}
        value={value === "" || value === undefined ? undefined : Number(value)}
        {...props}
      >
        <HeroNumberFieldGroupRoot>
          <HeroNumberFieldDecrementButtonRoot aria-label="Decrease" />
          <HeroNumberFieldInputRoot
            className={classNames("hr-input", sizeClass(size))}
          />
          <HeroNumberFieldIncrementButtonRoot aria-label="Increase" />
        </HeroNumberFieldGroupRoot>
      </HeroNumberFieldRoot>
      {description ? (
        <HeroDescriptionRoot className="hr-field-description">
          {description}
        </HeroDescriptionRoot>
      ) : null}
      {error ? <HeroFieldErrorRoot>{error}</HeroFieldErrorRoot> : null}
    </div>
  );
}

export function UiFileField({
  clearable: _clearable,
  onChange,
  value: _value,
  ...props
}: Omit<InputProps, "onChange" | "type" | "value"> & {
  clearable?: boolean;
  onChange?: (value: File | null) => void;
  value?: File | null;
}) {
  return (
    <FieldInput
      type="file"
      onChange={(event) => onChange?.(event.currentTarget.files?.[0] ?? null)}
      {...props}
    />
  );
}

function FieldInput({
  className,
  description,
  error,
  label,
  size,
  style,
  ...props
}: InputProps) {
  return (
    <div
      className={classNames("hr-field", className)}
      style={layoutStyle({ ...props, style })}
    >
      {label ? <HeroLabelRoot>{label}</HeroLabelRoot> : null}
      <HeroInputRoot
        className={classNames("hr-input", sizeClass(size))}
        {...props}
      />
      {description ? (
        <HeroDescriptionRoot className="hr-field-description">
          {description}
        </HeroDescriptionRoot>
      ) : null}
      {error ? <HeroFieldErrorRoot>{error}</HeroFieldErrorRoot> : null}
    </div>
  );
}

type TextareaProps = Omit<ComponentPropsWithoutRef<"textarea">, "size"> &
  LayoutProps & {
    description?: ReactNode;
    error?: ReactNode;
    label?: ReactNode;
    minRows?: number;
    size?: Size;
  };

export function UiTextarea({
  className,
  description,
  error,
  label,
  minRows,
  size,
  style,
  ...props
}: TextareaProps) {
  return (
    <div
      className={classNames("hr-field", className)}
      style={layoutStyle({ ...props, style })}
    >
      {label ? <HeroLabelRoot>{label}</HeroLabelRoot> : null}
      <HeroTextAreaRoot
        className={classNames("hr-input", "hr-textarea", sizeClass(size))}
        rows={minRows}
        {...props}
      />
      {description ? (
        <HeroDescriptionRoot className="hr-field-description">
          {description}
        </HeroDescriptionRoot>
      ) : null}
      {error ? <HeroFieldErrorRoot>{error}</HeroFieldErrorRoot> : null}
    </div>
  );
}

type UiSelectItem<T extends string = string> = {
  label: string;
  value: T;
};

type UiSelectProps<T extends string = string> = Omit<
  ComponentPropsWithoutRef<"select">,
  "data" | "onChange" | "size" | "value"
> &
  LayoutProps & {
    data?: ReadonlyArray<UiSelectItem<T> | T>;
    error?: ReactNode;
    label?: ReactNode;
    onChange?: (value: T | null) => void;
    placeholder?: string;
    size?: Size;
    value?: string | null;
  };

export function UiSelect<T extends string = string>({
  className,
  data = [],
  error,
  label,
  onChange,
  placeholder,
  size,
  style,
  value,
  ...props
}: UiSelectProps<T>) {
  return (
    <label
      className={classNames("hr-field", className)}
      style={layoutStyle({ ...props, style })}
    >
      {label ? <span>{label}</span> : null}
      <select
        className={classNames("hr-input", sizeClass(size))}
        value={value ?? ""}
        onChange={(event) =>
          onChange?.((event.currentTarget.value || null) as T | null)
        }
        {...props}
      >
        {placeholder ? <option value="">{placeholder}</option> : null}
        {data.map((item) => {
          const option =
            typeof item === "string" ? { label: item, value: item } : item;

          return (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          );
        })}
      </select>
      {error ? <small>{error}</small> : null}
    </label>
  );
}

type CheckboxProps = Omit<
  ComponentPropsWithoutRef<"input">,
  "onChange" | "type"
> & {
  checked?: boolean;
  label?: ReactNode;
  onChange?: (event: ChangeEvent<HTMLInputElement>) => void;
};

export function UiCheckbox({
  checked,
  className,
  disabled,
  label,
  onChange,
  ...props
}: CheckboxProps) {
  return (
    <HeroCheckboxRoot
      className={classNames("hr-check", className)}
      {...props}
      isDisabled={disabled}
      isSelected={checked}
      onChange={(checked: boolean) =>
        onChange?.({
          currentTarget: { checked },
          target: { checked },
        } as ChangeEvent<HTMLInputElement>)
      }
    >
      <span>{label}</span>
    </HeroCheckboxRoot>
  );
}

export function UiSwitch({
  checked,
  className,
  disabled,
  label,
  onChange,
  ...props
}: CheckboxProps) {
  return (
    <HeroSwitchRoot
      className={classNames("hr-switch", className)}
      {...props}
      isDisabled={disabled}
      isSelected={checked}
      onChange={(checked: boolean) =>
        onChange?.({
          currentTarget: { checked },
          target: { checked },
        } as ChangeEvent<HTMLInputElement>)
      }
    >
      <span aria-hidden="true" />
      {label ? <strong>{label}</strong> : null}
    </HeroSwitchRoot>
  );
}

type SegmentedControlProps<T extends string = string> = LayoutProps & {
  data: ReadonlyArray<UiSelectItem<T> | T>;
  onChange?: (value: T) => void;
  size?: Size;
  value?: T;
};

export function UiSegmented<T extends string = string>({
  className,
  data,
  onChange,
  style,
  value,
  ...props
}: SegmentedControlProps<T>) {
  return (
    <div
      className={classNames("hr-segmented", className)}
      style={layoutStyle({ ...props, style })}
    >
      {data.map((item) => {
        const option =
          typeof item === "string" ? { label: item, value: item } : item;

        return (
          <button
            className={option.value === value ? "active" : undefined}
            key={option.value}
            onClick={() => onChange?.(option.value as T)}
            type="button"
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

type UiDividerProps = LayoutProps & {
  label?: ReactNode;
  labelPosition?: "center" | "left" | "right";
};

export function UiDivider({
  className,
  label,
  style,
  ...props
}: UiDividerProps) {
  return (
    <HeroSeparatorRoot
      className={classNames("hr-divider", className)}
      style={layoutStyle({ ...props, style })}
    >
      {label ? <span>{label}</span> : null}
    </HeroSeparatorRoot>
  );
}

type AnchorProps = ComponentPropsWithoutRef<"a"> &
  LayoutProps & {
    component?: ElementType;
    size?: Size;
    to?: string;
  };

export function UiLink({
  children,
  className,
  component: Component = "a",
  size,
  style,
  ...props
}: AnchorProps) {
  if (Component !== "a") {
    return (
      <Component
        className={classNames("hr-anchor", sizeClass(size), className)}
        style={layoutStyle({ ...props, style })}
        {...props}
      >
        {children}
      </Component>
    );
  }

  return (
    <HeroLinkRoot
      className={classNames("hr-anchor", sizeClass(size), className)}
      style={layoutStyle({ ...props, style })}
      {...props}
    >
      {children}
    </HeroLinkRoot>
  );
}

type LoaderProps = {
  "aria-label"?: string;
  size?: Size;
};

export function UiSpinner({ size, ...props }: LoaderProps) {
  return (
    <HeroSpinnerRoot
      className={classNames("hr-loader", sizeClass(size))}
      size={size === "md" ? undefined : size}
      {...props}
    />
  );
}

type TableRootProps = Omit<ComponentPropsWithoutRef<"table">, "style"> &
  LayoutProps & {
    striped?: boolean;
    withRowBorders?: boolean;
    withTableBorder?: boolean;
  };

function TableRoot({
  className,
  striped,
  style,
  withRowBorders,
  withTableBorder,
  ...props
}: TableRootProps) {
  return (
    <table
      className={classNames(
        "hr-table",
        striped ? "hr-table-striped" : undefined,
        withRowBorders ? "hr-table-row-borders" : undefined,
        withTableBorder ? "hr-table-border" : undefined,
        className,
      )}
      style={layoutStyle({ ...props, style })}
      {...props}
    />
  );
}

function TableThead(props: ComponentPropsWithoutRef<"thead">) {
  return <thead {...props} />;
}

function TableTbody(props: ComponentPropsWithoutRef<"tbody">) {
  return <tbody {...props} />;
}

function TableTr({
  bg,
  style,
  ...props
}: ComponentPropsWithoutRef<"tr"> & LayoutProps) {
  return <tr style={layoutStyle({ bg, style })} {...props} />;
}

function TableTh(props: ComponentPropsWithoutRef<"th">) {
  return <th {...props} />;
}

function TableTd(props: ComponentPropsWithoutRef<"td">) {
  return <td {...props} />;
}

export const UiDataTable = Object.assign(TableRoot, {
  Tbody: TableTbody,
  Td: TableTd,
  Th: TableTh,
  Thead: TableThead,
  Tr: TableTr,
});

function fontSize(size: Size | undefined) {
  if (size === "xs") {
    return 12;
  }

  if (size === "sm") {
    return 14;
  }

  if (size === "lg") {
    return 18;
  }

  if (size === "xl") {
    return 20;
  }

  return typeof size === "number" ? size : undefined;
}

function radiusValue(radius: Size | undefined) {
  if (radius === "sm") {
    return 6;
  }

  if (radius === "md" || radius === undefined) {
    return 8;
  }

  if (radius === "lg") {
    return 12;
  }

  if (radius === "xl") {
    return 16;
  }

  return radius;
}

function sizeClass(size: Size | undefined) {
  return typeof size === "string" ? `hr-size-${size}` : undefined;
}

function toneClass(tone: Tone | undefined) {
  return tone ? `hr-tone-${tone}` : undefined;
}

function variantClass(variant: Variant | undefined) {
  return variant ? `hr-variant-${variant}` : undefined;
}
