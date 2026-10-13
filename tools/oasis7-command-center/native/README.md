# Administrator bridge prerequisite

Administrator helper integration is held at the user request. No helper is bundled or registered, no authorization is requested and no privileged operation is exposed by the App. Existing ordinary installed caller operations remain separate.

The read-only development machine probe `security find-identity -v -p codesigning` reported zero valid identities. Future native acceptance needs a matching signed App/helper, macOS 13 SMAppService approval, authenticated XPC peer identity, finite Authorization Services rights and independently trusted signer gate/catalog. The source implementation and runtime acceptance must be completed before enabling management controls.
