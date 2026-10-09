<script lang="ts">
  // The LogRocket logo at the right end of the page header, linking to the privacy page: the site
  // records visits with LogRocket (`lib/logrocket.ts`), and the page says what is recorded. The
  // logo is LogRocket's rocket mark, taken from the logotype in the header of logrocket.com (a
  // trademark of LogRocket, Inc.; credited in `README.md`), inline so the page loads nothing extra. Like the
  // other logos (`BuiltWith.svelte`) it is 1em tall, the height of the tagline's text, and it takes
  // the text color, so it shows in light and dark mode.
  interface Props {
    /** The privacy page's address. */
    href: string;
  }
  let { href }: Props = $props();

  const label = 'Privacy: this site records visits with LogRocket';
</script>

<!-- A new tab, so following the link doesn't leave the app (and lose the open image). The logo is
     only a picture (`aria-hidden`); the link's name for screen readers and its tooltip say what it
     is about. -->
<a
  class="logrocket"
  {href}
  target="_blank"
  rel="noopener noreferrer"
  title={label}
  aria-label={label}
>
  <svg viewBox="0 0 18.2 28" aria-hidden="true">
    <path
      fill="currentColor"
      fill-rule="evenodd"
      clip-rule="evenodd"
      d="M6.066 3.156A10.53 10.53 0 0 1 9.122 0a10.294 10.294 0 0 1 3.016 3.094 15.59 15.59 0 0 1 2.93 9.777c.637.513 1.293 1.006 1.918 1.53a3.46 3.46 0 0 1 1.104 3.189c-.302 1.457-.592 2.918-.911 4.372a1.214 1.214 0 0 1-1.848.61c-1.027-.825-2.027-1.678-3.05-2.504a4.684 4.684 0 0 1-2.891 1.255 4.678 4.678 0 0 1-3.385-1.22c-.735.541-1.419 1.191-2.138 1.772-.315.31-.666.58-1.046.806a1.215 1.215 0 0 1-1.603-.785c-.329-1.422-.672-2.839-.99-4.263a3.453 3.453 0 0 1 1.163-3.321c.559-.45 1.125-.893 1.694-1.331.159-.08.08-.261.087-.401a15.615 15.615 0 0 1 2.9-9.42m1.007 4.402a2.395 2.395 0 0 0 .21 3.173 2.636 2.636 0 0 0 3.603.075 2.398 2.398 0 0 0 .636-2.634 2.55 2.55 0 0 0-2.14-1.59 2.6 2.6 0 0 0-2.31.974"
    />
    <path
      fill="currentColor"
      d="M5.712 23.082a.605.605 0 0 1 .896-.485 5.778 5.778 0 0 0 5.03 0 .61.61 0 0 1 .896.45c.005.89.005 1.78 0 2.67a.602.602 0 0 1-.94.436c-.267-.226-.508-.48-.764-.719-.407.762-.789 1.534-1.199 2.294a.61.61 0 0 1-1.012.006c-.41-.761-.79-1.538-1.206-2.299-.253.24-.494.494-.761.72a.603.603 0 0 1-.94-.442c-.007-.878 0-1.756 0-2.634M9.102 10.259a1.22 1.22 0 0 0 1.248-1.192v-.008a1.221 1.221 0 0 0-1.24-1.2h-.008A1.22 1.22 0 0 0 7.855 9.05v.008a1.22 1.22 0 0 0 1.24 1.2h.007Z"
    />
  </svg>
</a>

<style>
  .logrocket {
    display: flex;
    color: var(--fg);
  }
  /* 1em tall like the other logos; the width follows the logo's proportions. */
  svg {
    height: 1em;
    width: auto;
    aspect-ratio: 18.2 / 28;
  }
</style>
