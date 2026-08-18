import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import SwatchPicker from "./SwatchPicker.vue";
import { PALETTE } from "../lib/format";

describe("SwatchPicker", () => {
  it("marks the selected swatch", () => {
    const wrapper = mount(SwatchPicker, { props: { modelValue: PALETTE[2] } });
    const buttons = wrapper.findAll("button.sw");
    expect(buttons).toHaveLength(PALETTE.length);
    expect(buttons[2].classes()).toContain("sel");
  });

  it("emits update:modelValue with the clicked hex", async () => {
    const wrapper = mount(SwatchPicker, { props: { modelValue: PALETTE[0] } });
    await wrapper.findAll("button.sw")[3].trigger("click");
    expect(wrapper.emitted("update:modelValue")?.[0]).toEqual([PALETTE[3]]);
  });
});
