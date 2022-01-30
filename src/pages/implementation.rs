use yew::{function_component, html};

use super::PageMetadata;

#[function_component(Implementation)]
pub fn implementation_page() -> Html {
    let metadata = PageMetadata {
        title: "Implementation details and security models".to_owned(),
        description: "Elastic Poll is a fully contained WASM web app allowing to hold polls \
            in a cryptographically secure and private manner. \
            This page describes how the app works and its limitations."
            .to_owned(),
        is_root: false,
    };

    html! {
        <>
            { metadata.view() }
            <p class="lead">
                { "Elastic poll is a web app that allows organizing single-choice and \
                multi-choice polls that combine privacy and universal verifiability with \
                the help of applied cryptography." }
            </p>
            <div class="alert alert-warning">
                <h4 class="alert-heading">{ "Use at your own risk!" }</h4>
                <p class="mb-0">
                    { "Cryptography behind the app was not independently audited, in particular \
                    against side-channel (e.g., timing) attacks. The app is provided without \
                    any warranty or liability as specified in the " }
                    <a href="https://www.apache.org/licenses/LICENSE-2.0"
                        class="alert-link">
                        { "Apache 2.0 license" }
                    </a>
                    { "." }
                </p>
            </div>
            <p>
                { "The polling process consists of 4 stages: specifying a poll, selecting \
                participants, submitting votes and tallying results. Because of serverless \
                architecture, there is no tamper-proof bulletin available to all participants \
                where these stages are performed. As such, " }
                <strong>
                    { "participants need to use an external bulletin of their choosing," }
                </strong>
                { " such as a Telegram group or a Slack channel \
                (or maybe a blockchain). The app does provide reference values allowing \
                to understand whether the necessary data is synced among participants." }
            </p>

            <h4>{ "Voting" }</h4>
            <p>
                { "The app uses " }
                <a href="https://en.wikipedia.org/wiki/ElGamal_encryption">
                    { "ElGamal encryption" }
                </a>
                { " to encrypt votes. A Boolean vote (i.e., 0 or 1) is encrypted separately \
                for each option for the shared tallying key (see " }
                <a href="#participants">{ "below" }</a>
                { " how this key is constructed). The app uses " }
                <a href="https://ristretto.group/">{ "Ristretto255" }</a>
                { ", a prime-order elliptic curve group obtained by transforming Curve25519, \
                as a prime-order group underpinning ElGamal encryption." }
            </p>
            <p>
                { "Together with the option ciphertexts, a vote contains a zero-knowledge proof \
                that each of them encrypts 0 or 1; this proof is based on " }
                <a href="https://raw.githubusercontent.com/Blockstream/borromean_paper/master/borromean_draft_0.01_34241bb.pdf">
                    { "Borromean ring signatures" }
                </a>
                { " by Maxwell and Poelstra, repurposed to work on ElGamal ciphertexts instead \
                of public keys. See " }
                <a href="https://slowli.github.io/elastic-elgamal/elastic_elgamal/struct.RingProof.html">
                    <code>{ "RingProof" }</code>
                </a>
                { " docs from the " }
                <a href="https://github.com/slowli/elastic-elgamal">
                    <code>{ "elastic-elgamal" }</code>
                </a>
                { " crate for more technical details how this proof is constructed and verified." }
            </p>
            <p>
                { "For single-choice polls, the vote additionally contains a zero-knowledge proof \
                that exactly 1 option is selected, i.e., the option ciphertexts sum up to \
                a ciphertext of 1. This is a standard discrete log equality proof (aka \
                Chaum–Pedersen protocol). For multi-choice polls, this proof is not necessary." }
            </p>
            <p>{ "ElGamal encryption is additively homomorphic; the sum of ciphertexts \
            for the same public key encrypts the sum of the corresponding plaintexts \
            for the same key. As such, vote tallying is straightforward – the ciphertexts are \
            added for each option and then decrypted, allowing to restore the number of votes \
            submitted for each option. It is possible to prove the validity of decryption without \
            disclosing the (private) decryption key. Indeed, it suffices to provide \
            a single group element – the result \
            of the Diffie–Hellman exchange between the decryption key and the random group element \
            from the ElGamal ciphertext. The validity of this element can be proven via \
            a standard discrete log equality proof." }</p>

            <h4 id="participants">{ "Participants" }</h4>
            <p>
                { "All participants are simultaneously both voters and talliers. A participant \
                can skip voting, but cannot skip tallying (i.e., involvement of all talliers is \
                required to determine final results). " }
                <strong>
                    { "If any tallier refuses to cooperate, the poll is stalled indefinitely." }
                </strong>
            </p>
            <p>{ "Such design is motivated by 2 factors:" }</p>
            <ul>
                <li>
                    <strong>{ "Simplicity of initialization:" }</strong>
                    { " N-of-N tallier scheme is significantly easier to initialize than a generic \
                        M-of-N scheme. The latter would require performing a verifiable secret \
                        sharing or distributed key generation protocol." }
                </li>
                <li>
                    <strong>{ "Strong privacy guarantees:" }</strong>
                    { " It is impossible to decrypt any separate vote without the voter’s consent." }
                </li>
            </ul>
            <p>
                { "Each participant has a Ristretto255 keypair to sign their votes \
                (i.e., prove that the vote \
                comes from an eligible voter), and to submit a tallying share. \
                (Tallying shares do not need additional authentication since they contain \
                a sufficient zero-knowledge proof of authenticity.) \
                When combined, shares from all participants (and no less than all participants!) \
                would allow to decrypt vote results. The shared public key used to encrypt \
                votes is the simple sum of all participants’ public keys. Correspondingly, \
                tallying shares (i.e., a vector of Diffie–Hellman multiplications \
                of the participant’s private key with the random elements \
                of all the tallied option ciphertexts) \
                are combined by summing as well. To prevent rogue key \
                attacks, a participant application contains, along with a public key, \
                a zero-knowledge proof of ownership of the corresponding secret key." }
            </p>
        </>
    }
}
