/**
 * Perf cases: renderString with path-keyed in-memory templates (macro libraries), autoescape off,
 * throwOnUndefined on — typical of LLM system-prompt style pipelines without tying to any product.
 */

/** @returns {{ autoescape: boolean, throwOnUndefined: boolean, templateMap?: Record<string, string> }} */
export function inlineMacroLibraryEnv(templateMap = undefined) {
  const base = {
    autoescape: false,
    throwOnUndefined: true,
  }
  if (templateMap != null) {
    return { ...base, templateMap }
  }
  return base
}

const MACRO_SHARED_BULK = `{% macro m1() %}1{% endmacro %}
{% macro m2() %}2{% endmacro %}
{% macro m3() %}3{% endmacro %}
{% macro m4() %}4{% endmacro %}
{% macro m5() %}5{% endmacro %}
{% macro m6() %}6{% endmacro %}
{% macro m7() %}7{% endmacro %}
{% macro m8() %}8{% endmacro %}
{% macro used(x) %}U{{ x }}{% endmacro %}
`

const MACRO_OTHER_BULK = `{% macro symA() %}A{% endmacro %}
{% macro symB() %}B{% endmacro %}
`

const MACRO_STYLE_KWARGS = `{% macro styleMirrorBlock(subject, cap, tagSuffix="", showExtra=false, defaultTag="", useOptionalBlock=true) -%}
{% set effTag = tagSuffix if tagSuffix else subject %}
{% set resolvedTag = defaultTag if defaultTag else "DEFAULT_INNER_" ~ effTag %}
<STYLE>
<MIRROR_{{ effTag }}>
as {{ subject }} cap {{ cap }}{% if showExtra %} extra{% endif %}
</MIRROR_{{ effTag }}>
<{{ resolvedTag }}>inner</{{ resolvedTag }}>
{% if useOptionalBlock %}
<OPT>{{ cap }}</OPT>
{% endif %}
</STYLE>
{%- endmacro %}
`

const MACRO_GROUPED_NOTES = `{% macro formatGroupedNotes(shortGroups=[], longRows=[]) -%}
<GROUPED_NOTES>
{% if longRows | length > 0 -%}
Noted {% if longRows | length > 1 %}entries{% else %}one entry{% endif %}:
{% for row in longRows -%}
- {{ row.body }}{% if row.hint %} (hint {{ row.hint }}){% endif %}
{% endfor -%}
{% endif -%}
{% if longRows | length > 0 and shortGroups | length > 0 -%}{{ "\\n" -}}{% endif -%}
{% for group in shortGroups -%}
{{ group.title }}
{% for item in group.rows -%}
- {{ item.k }}: {{ item.v }}
{% endfor -%}
{% if not loop.last -%}{{ "\\n" -}}{% endif -%}
{% endfor -%}
</GROUPED_NOTES>
{%- endmacro %}
`

const MACRO_BULLETS = `{% macro emitLines(body) %}{{ body }}{% endmacro %}
`

function groupedNotesContextSmall() {
  return {
    ctxShortGroups: [
      {
        title: 'Block A',
        rows: [
          { k: 'a', v: '1' },
          { k: 'b', v: '2' },
        ],
      },
    ],
    ctxLongRows: [{ body: 'alpha', hint: 'h0' }],
  }
}

function groupedNotesContextLarge() {
  const ctxLongRows = Array.from({ length: 24 }, (_, i) => ({
    body: `row-${i}`,
    hint: i % 3 === 0 ? `h${i}` : '',
  }))
  const ctxShortGroups = Array.from({ length: 12 }, (_, g) => ({
    title: `Section ${g}`,
    rows: Array.from({ length: 6 }, (_, j) => ({
      k: `K${g}-${j}`,
      v: `V${g}-${j}`,
    })),
  }))
  return { ctxShortGroups, ctxLongRows }
}

const TPL_FROM_HEAVY = `{% from "macros/sharedBulk.njk" import m1, m2, m3, m4, m5, m6, m7, m8, used %}
{% from "macros/otherBulk.njk" import symA, symB %}
{{ used(msg) }}|{{ symA() }}{{ symB() }}
`

const TPL_STYLE = `{% from "macros/styleKwargs.njk" import styleMirrorBlock %}
{{ styleMirrorBlock(
  subject="the user",
  cap="keep it short",
  tagSuffix="SUBJECT_TAG",
  showExtra=false,
  defaultTag="INNER",
  useOptionalBlock=true
) }}
`

const TPL_GROUPED_NOTES = `{% from "macros/groupedNotes.njk" import formatGroupedNotes %}
{{ formatGroupedNotes(shortGroups=ctxShortGroups, longRows=ctxLongRows) }}
`

const TPL_SET_PIPELINE = `{%- set needAlpha = not hasAlpha -%}
{%- set needBeta = not hasBeta -%}
{%- set combined = hasAlpha or hasBeta -%}
{%- set blurb -%}Intro line{% if needBeta %} (beta){% endif %}{%- endset -%}
{%- set tier -%}{% if combined %}T1{% elif needAlpha %}T2{% else %}T0{% endif %}{%- endset -%}
{{ blurb }}|{{ tier }}|{{ combined }}
`

const TPL_JOIN_BULLETS = `{% from "macros/bullets.njk" import emitLines %}
{%- set LINES = [
  "alpha",
  "beta",
  "gamma"
] | join("\\n") -%}
{{ emitLines(LINES) }}
`

export function inlineMacroLibraryCases() {
  const mapFromHeavy = {
    'macros/sharedBulk.njk': MACRO_SHARED_BULK,
    'macros/otherBulk.njk': MACRO_OTHER_BULK,
  }

  const mapStyle = {
    'macros/styleKwargs.njk': MACRO_STYLE_KWARGS,
  }

  const mapNotes = {
    'macros/groupedNotes.njk': MACRO_GROUPED_NOTES,
  }

  const mapBullets = {
    'macros/bullets.njk': MACRO_BULLETS,
  }

  return [
    {
      name: 'inmem_macro_baseline_small',
      template: 'hello',
      context: {},
      env: inlineMacroLibraryEnv(),
    },
    {
      name: 'inmem_macro_from_import_heavy',
      template: TPL_FROM_HEAVY,
      context: { msg: 'ok' },
      env: inlineMacroLibraryEnv(mapFromHeavy),
    },
    {
      name: 'inmem_macro_style_kwarg_shape',
      template: TPL_STYLE,
      context: {},
      env: inlineMacroLibraryEnv(mapStyle),
    },
    {
      name: 'inmem_macro_nested_groups_small',
      template: TPL_GROUPED_NOTES,
      context: groupedNotesContextSmall(),
      env: inlineMacroLibraryEnv(mapNotes),
    },
    {
      name: 'inmem_macro_nested_groups_large',
      template: TPL_GROUPED_NOTES,
      context: groupedNotesContextLarge(),
      env: inlineMacroLibraryEnv(mapNotes),
    },
    {
      name: 'inmem_macro_set_pipeline',
      template: TPL_SET_PIPELINE,
      context: {
        hasAlpha: true,
        hasBeta: false,
      },
      env: inlineMacroLibraryEnv(),
    },
    {
      name: 'inmem_macro_join_lists',
      template: TPL_JOIN_BULLETS,
      context: {},
      env: inlineMacroLibraryEnv(mapBullets),
    },
  ]
}
