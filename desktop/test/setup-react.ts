import { mock } from "bun:test";
import React from "react";

// @lingui/react/macro 모킹 (빌드 시점 매크로이므로 런타임에서 직접 사용 불가)
mock.module("@lingui/react/macro", () => ({
  Trans: ({ children }: { children: React.ReactNode }) =>
    React.createElement(React.Fragment, null, children),
  useLingui: () => ({
    t: (strings: TemplateStringsArray, ...values: unknown[]) =>
      String.raw({ raw: strings }, ...values),
  }),
}));
