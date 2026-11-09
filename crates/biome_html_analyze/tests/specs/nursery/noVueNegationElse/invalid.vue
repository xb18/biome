<!-- should generate diagnostics -->
<template>
  <div v-if="!foo" class="primary">Content</div>
  <span v-else class="secondary">Alternative</span>

  <div v-if="!(foo && bar)" id="negated">Negated condition</div>
  <span v-else id="positive">Otherwise</span>

  <div v-if="a != b">Not equal</div>
  <div v-else>Equal</div>

  <div v-if="a !== b">Strictly not equal</div>
  <div v-else>Strictly equal</div>

  <div v-if="foo" class="first">First</div>
  <span v-else-if="!bar" class="second">Second</span>
  <p v-else class="default">Default</p>

  <div v-if="!a" data-first>First</div>
  <span v-else-if="!b" data-second>Second</span>
  <p v-else data-default>Default</p>

  <div v-if="(!foo)">Parenthesized</div>
  <div v-else>Otherwise</div>

  <img v-if="!src" alt="placeholder" />
  <img v-else :src="src" alt="image" />

  <div v-if="!foo">First</div>
  <!-- a comment between the branches -->
  <div v-else>Second</div>

  <MyComponent v-if="!ready" />
  <div v-else>Ready</div>

  <div v-if="!outer" class="outer-if">
    <span v-if="!inner" class="inner-if">Inner if content</span>
    <p v-else class="inner-else">Inner else content</p>
  </div>
  <section v-else class="outer-else">
    <span v-if="!nested" class="nested-if">Nested if content</span>
    <p v-else class="nested-else">Nested else content</p>
  </section>
</template>
