---
version: alpha
name: Project Name
description: Frontend UI Style & Theme Guide
colors:
  primary-500: "#3B82F6"
  primary-600: "#2563EB"
  primary-700: "#1D4ED8"
  secondary-500: "#6366F1"
  secondary-600: "#4F46E5"
  neutral-background: "#FFFFFF"
  neutral-surface: "#F9FAFB"
  neutral-border: "#E5E7EB"
  neutral-text-primary: "#111827"
  neutral-text-secondary: "#6B7280"
  neutral-text-disabled: "#9CA3AF"
  semantic-success: "#22C55E"
  semantic-warning: "#F59E0B"
  semantic-error: "#EF4444"
  semantic-info: "#0EA5E9"
  dark-background: "#0F172A"
  dark-surface: "#020617"
  dark-text-primary: "#F9FAFB"
  dark-text-secondary: "#CBD5E1"
  dark-divider: "#1E293B"
spacing:
  "4": "4px"
  "8": "8px"
  "12": "12px"
  "16": "16px"
  "24": "24px"
  "32": "32px"
  "40": "40px"
  "48": "48px"
---

## Overview

This document defines the visual system, theming rules, and UI standards for the frontend. All UI work MUST comply with this guide unless explicitly overridden.

### Design Principles

* UI MUST be simple, consistent, and predictable.
* Reuse MUST be preferred over customization.
* Visual decisions MUST be driven by roles and tokens, never by ad-hoc values.
* Accessibility, responsiveness, and performance are first-class requirements.

## Colors

Colors MUST be defined in the theme and consumed by role. Raw hex values MUST NOT appear in widgets. Error/warning/success colors MUST NOT be reused for branding. Any new color role MUST be added to the theme and documented.

### Core Brand
Primary and secondary colors MUST be used only for key actions and emphasis.

### Neutral
Neutral colors MUST form the base of the UI.

### Semantic / Status
Status colors MUST be used only to express state, never decoration.

### Dark Mode
All colors MUST meet contrast requirements in both themes.

## Typography

* Text styles MUST come from theme configuration.
* Hardcoded font sizes, weights, or line heights MUST NOT be used in widgets.
* Typography roles MUST be limited and reused (display, title, body, label).
* Text overflow MUST be handled explicitly.
* Wide layouts MUST constrain line length for readability.

## Layout

* Layout MUST use an **8pt spacing system**: 4, 8, 12, 16, 24, 32, 40, 48.
* Screen padding MUST be 16 on mobile, 24 on tablet/desktop.
* Vertical rhythm MUST be consistent; arbitrary spacing values MUST NOT be used.
* Safe areas MUST be respected unless explicitly required otherwise.

## Components

* UI MUST be composed from reusable components in the shared `ui/` layer.
* Screens MUST NOT define custom buttons, inputs, or cards if a shared one exists.
* Components MUST expose minimal, stable APIs.
* Loading, empty, and error states MUST be standardized and reused.

## Do's and Don'ts

* **Theming:** UI MUST consume colors via context theme or extensions. Widgets MUST NOT assume light or dark mode.
* **Interaction:** Interactive elements MUST provide visual feedback. Destructive actions MUST be clearly labeled and confirmed. Navigation MUST be consistent.
* **Forms:** Inputs MUST include labels and accessible hints. Validation MUST be user-friendly. Errors MUST NOT expose internal technical messages.
* **Responsiveness:** UI MUST adapt across phone, tablet, desktop, and web. Large screens MUST use constrained widths and multi-column layouts.
* **Motion:** Animations MUST be subtle. Loading indicators MUST appear for operations >300ms. Skeletons SHOULD be used for list-heavy screens.
* **Accessibility:** Touch targets MUST be >=44x44 logical pixels. Semantics MUST be provided for key elements. Color MUST NOT be the only indicator of state.
* **Performance:** Rebuild scope MUST be minimized using widget composition and const. Heavy work MUST NOT run on the UI thread.
* **Definition of Done:** A screen is complete only if it handles loading/empty/error/success states, is responsive/accessible, uses themed styles, passes tests, and introduces no hardcoded visual values.
