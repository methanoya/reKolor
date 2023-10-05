<script lang="ts">
  import { htmlColor } from './utils';

  export let stripe: PickingStripeType;

  import { Toast, ToastBody, ToastHeader } from 'sveltestrap';
  import _ from 'lodash';
  import type { PickingStripeType } from './types';
  import { pickingStripes } from './store';

  let width: number, height: number;
  let original_color: string;
  let palette_color: string;
  let caption: string = '?';

  $: {
    height = width / 8;
    const { picked, original, nominal } = stripe;
    original_color = htmlColor(original);
    palette_color = htmlColor(nominal);
    caption = `rgb (${picked.r}, ${picked.g}, ${picked.b}),   alpha(${
      picked.a == 1 ? 1 : picked.a.toFixed(3)
    })`;
  }

  const toggle = () => {
    console.log('Delete stripe', caption);
    pickingStripes.update((list) =>
      _.filter(list, ({ original }) => !_.isEqual(stripe.original, original)),
    );
  };

  const onMouse = (event) => {
    console.log('*', caption);
  };

  const onKeypress = (event) => {};
</script>

<Toast>
  <ToastHeader {toggle}>{@html caption}</ToastHeader>
  <ToastBody>
    <div class="container" bind:clientWidth={width}>
      <div class="title">
        <div class="original-title">{original_color}</div>
        <div class="palette-title"></div>
      </div>
      <div class="picker" style="--height:{height};">
        <div class="original" style="--original_color:{original_color}">
          &nbsp;
        </div>
        <div
          class="palette"
          style="--palette_color:{palette_color}"
          on:click={onMouse}
          on:keypress={onKeypress}
        >
          &nbsp;
        </div>
      </div>
      <div class="title">
        <div class="original-title"></div>
        <div class="palette-title">{stripe?.nominalName ?? 'none'}</div>
      </div>
    </div>
  </ToastBody>
</Toast>

<style>
  .container {
    overflow: hidden;
  }

  .title {
    overflow: hidden;
    width: 100%;
  }

  .picker {
    overflow: hidden;
    width: 100%;
    height: calc(var(--height) * 1px);
  }

  .original {
    float: left;
    background-color: var(--original_color);
    width: 50%;
    height: 100%;
  }

  .original-title {
    float: left;
    width: 50%;
  }

  .palette {
    text-align: right;
    float: right;
    background-color: var(--palette_color);
    width: 50%;
    height: 100%;
  }

  .palette-title {
    text-align: right;
    float: right;
    width: 50%;
  }
</style>
