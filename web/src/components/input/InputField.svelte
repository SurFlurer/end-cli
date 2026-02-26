<script lang="ts">
  type InputType = "text" | "number";

  interface Props {
    value: string | number;
    type?: InputType;
    id?: string;
    min?: number | string;
    max?: number | string;
    step?: number | string;
    placeholder?: string;
    disabled?: boolean;
    ariaLabel?: string;
    oninput?: (event: Event) => void;
  }

  let {
    value,
    type = "text",
    id,
    min,
    max,
    step,
    placeholder,
    disabled = false,
    ariaLabel,
    oninput,
  }: Props = $props();
</script>

<input
  {id}
  {type}
  {min}
  {max}
  {step}
  {placeholder}
  {disabled}
  aria-label={ariaLabel}
  {value}
  oninput={oninput}
/>

<style>
  input {
    border: 1px solid var(--control-border);
    border-radius: var(--radius-sm);
    height: var(--control-size);
    padding: 0 12px;
    background: var(--panel-strong);
    color: inherit;
    font: inherit;
    line-height: 1.2;
    width: 100%;
    min-width: 0;
  }

  /* Hide number input spinners (Chrome/Safari/Edge + Firefox) */
  input[type="number"] {
    appearance: textfield;
    -moz-appearance: textfield;
  }

  input[type="number"]::-webkit-outer-spin-button,
  input[type="number"]::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  @media (hover: hover) and (pointer: fine) {
    input:hover {
      box-shadow: var(--focus-ring);
    }
  }

  input:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
</style>
