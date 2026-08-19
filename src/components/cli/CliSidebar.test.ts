import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";
import CliSidebar from "./CliSidebar.vue";
import { useCliStore } from "../../stores/cli";
import type { CliProfile } from "../../lib/types";

function makeProfile(overrides: Partial<CliProfile>): CliProfile {
  return {
    name: "Test",
    slug: "test",
    config_dir: "/tmp/test",
    launcher_path: "/tmp/test.command",
    logged_in: true,
    account_email: "test@example.com",
    data_size: "0B",
    created: 0,
    last_active: null,
    auth_mode: "oauth",
    provider_model: null,
    token_expires_at: null,
    refresh_expires_at: null,
    subscription_type: null,
    rate_limit_tier: null,
    session_usage_pct: null,
    weekly_usage_pct: null,
    ...overrides,
  };
}

describe("CliSidebar", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the account list", () => {
    const store = useCliStore();
    store.cliProfiles = [makeProfile({ name: "Work CLI", slug: "work-cli" }), makeProfile({ name: "Personal CLI", slug: "personal-cli" })];
    const wrapper = mount(CliSidebar);
    const items = wrapper.findAll("li.cli-account-item");
    expect(items).toHaveLength(2);
    expect(items[0].text()).toContain("Work CLI");
  });

  it("selects an account on click and on Enter", async () => {
    const store = useCliStore();
    store.cliProfiles = [makeProfile({ name: "Work CLI", slug: "work-cli" }), makeProfile({ name: "Personal CLI", slug: "personal-cli" })];
    const wrapper = mount(CliSidebar);
    const items = wrapper.findAll("li.cli-account-item");

    await items[1].trigger("click");
    expect(store.cliSelected).toBe("personal-cli");

    await items[0].trigger("keydown.enter");
    expect(store.cliSelected).toBe("work-cli");
  });
});
